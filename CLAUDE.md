# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Contexto

3dnator é um CAD paramétrico em Rust (licença MIT), no **protótipo 0.3.0**: rascunhos com restrições, histórico paramétrico sobre OCCT, montagem, folhas técnicas e exportação STL/STEP. As pendências conhecidas estão em `ROADMAP.md` (seção Situação atual); o restante do README descreve o escopo aprovado, não necessariamente entregue.

As convenções obrigatórias (idioma, arquitetura, erros, versões, commits) estão em `AGENTS.md`; siga-as. Em resumo: identificadores em inglês, comentários e documentação em português, textos de interface em português via catálogo de tradução (`apps/prototype/src/i18n.rs`), CHANGELOG.md atualizado a cada mudança, commits descritivos em português, nada de push sem autorização.

Uma sessão do Codex pode estar trabalhando neste mesmo diretório. Antes de editar, verifique `pgrep -a codex` (o processo `codex` da TUI, não só o daemon `app-server`) e arquivos alterados recentemente; antes de cada commit, confira o `git status`. O sandbox do Codex não grava em `.git`, então os commits ficam com quem tem acesso ao repositório.

## Comandos

Requer Rust 1.95+ (edition 2024). Use `--locked`, como na CI.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# Um crate ou um teste específico
cargo test -p nator-sketch --locked
cargo test -p nator-project-io --locked rejects_truncation

# Editor (egui/eframe + wgpu); sem `occt`, não constrói sólidos
cargo run -p nator-prototype --features occt

# Ensaios sem janela (diretórios de saída devem ser novos)
cargo run -p nator-prototype --no-default-features -- --validate-project target/check/project
cargo run -p nator-prototype --no-default-features -- --validate-sketch target/check/sketch
cargo run -p nator-prototype -- --validate-editor target/check/editor
cargo run -p nator-prototype -- --render-project-preview PROJETO.3dnator target/check/editor.png
```

### Núcleo OCCT

A feature `occt` compila `crates/kernel/native/bridge.cpp` via CMake e vincula o OCCT 7.x (alvo 7.6 a 7.9). Configure `OpenCASCADE_DIR` (pasta com `OpenCASCADEConfig.cmake`) ou `CMAKE_PREFIX_PATH`, e deixe as bibliotecas no carregador (`LD_LIBRARY_PATH`; no Windows, DLLs no PATH). Nesta máquina, sem sudo, o pacote oficial do Arch foi extraído em `target/claude-occt-sdk/root/usr`; use um diretório de build separado para não invalidar o cache dos builds sem OCCT:

```sh
SDK=$PWD/target/claude-occt-sdk/root/usr
export OpenCASCADE_DIR=$SDK/lib/cmake/opencascade LD_LIBRARY_PATH=$SDK/lib CARGO_TARGET_DIR=$PWD/target/claude-occt
cargo clippy --workspace --all-targets --features nator-prototype/occt --locked -- -D warnings
cargo test --workspace --features nator-prototype/occt --locked      # inclui kernel/tests e commands/tests *_occt.rs
cargo run -p nator-prototype --features occt -- --validate target/check/occt          # primitivas, STEP/STL
cargo run -p nator-prototype --features occt -- --validate-part target/check/part     # critério da etapa 2
cargo run --release -p nator-prototype --features occt -- --benchmark target/check/bench   # GPU integrada; NATOR_BENCH_GPU=high
```

`cargo clean` apaga o SDK extraído; baixe de novo do mirror em `/etc/pacman.d/mirrorlist` conferindo o SHA-256 com `/var/lib/pacman/sync/extra.db`.

## Lints do workspace

Definidos em `Cargo.toml` e herdados por todos os crates:

- `unsafe_code = "deny"`: a única exceção é `crates/kernel/src/native.rs`. Todo bloco `unsafe` leva comentário `// SAFETY:`.
- Clippy nega `unwrap_used`, `expect_used` e `panic`, inclusive em testes; os testes retornam `Result` e usam `?`.
- Código só usado com `occt` precisa compilar sem avisos também sem a feature (veja o `cfg_attr(..., allow(dead_code))` em `kernel/src/lib.rs`).

## Arquitetura

`ARCHITECTURE.md` define os módulos e limites de dependência. Crates atuais:

| Crate | Papel |
| --- | --- |
| `geometry` | `Point3` (mm), `Mesh` validada e fundida (`welded`), topologia e arestas vivas (`edge_analysis`), `Placement` (rotação X→Y→Z) |
| `sketch` | Rascunhos, solver (graus de liberdade, conflito, redundância), ferramentas e regiões estáveis (`profiles::regions`/`find_region`) |
| `kernel` | Fronteira do OCCT: `Solid`, nomes de faces (`naming.rs`), perfis de furo (`hole.rs`) |
| `document` | Peças, corpos, instâncias, grupos, parâmetros com expressões, histórico (`features.rs`), seleção por clique (`selection.rs`) |
| `commands` | Comandos atômicos e desfazer/refazer por snapshots; recálculo (`features.rs`); consultas `measure` e `export` |
| `project_io` | STL e pacote `.3dnator` (versão 7, com migração), salvamento atômico e recuperação |
| `renderer` | Câmera ortográfica e `Viewport` wgpu que renderiza para textura; triangulação e arestas vivas no shader |
| `drawing` | Folhas técnicas a partir da malha: vistas, cotas, corte A–A hachurado, PNG/PDF |
| `apps/prototype` | Binário: CLI de ensaios, editor eframe (`desktop/project_editor/*`), i18n, benchmark |

### Referências estáveis (não volte a usar índices)

Regiões são identificadas pelas entidades do contorno; faces pelo nome de origem (`b{corpo}:e{entidade}`, `:start`, `:end`, `:round`, `:hole{i}:{papel}`, nomes herdados); arestas pelo par de faces adjacentes. A ponte C++ só informa a origem de cada face (trios tipo/entrada/índice); a nomeação fica em `kernel/src/naming.rs`. Seleções por índice (`RegionRef::Index`, `EdgeRef::Index`, `FaceRef::Index`) existem para projetos antigos e são convertidas em `commands::features::normalize`. Referência perdida ou ambígua deve falhar com erro explícito: nunca escolha outra geometria no lugar. O `MakeRevol` do OCCT 7.9 não informa faces planas em `Generated()`; a ponte as identifica geometricamente (`match_planar_revolution_faces`).

### Integração OCCT

- Camadas: `bridge.cpp` (único código com tipos OCCT; ABI C `nator_*`, exceções viram `false`/nullptr e `nator_last_error`) → `native.rs` (único módulo com FFI) → `Solid`. Nenhum tipo OCCT sai do crate `kernel`.
- `Solid` é `!Send`/`!Sync` (`PhantomData<Rc<()>>`): objetos nativos ficam na thread que os criou. Para cruzar threads, use bytes B-rep + nomes de faces.
- `build.rs` só age com `occt`; compilação cruzada é rejeitada. A ordem dos componentes no `find_package` importa (Visualization antes de ApplicationFramework).
- O messenger padrão do OCCT é silenciado na ponte, porque o stdout da CLI carrega relatórios JSON.
- Sem `occt`, operações CAD falham via `nator_kernel::require_available()`. Não crie núcleo alternativo nem simule resultados.

### Documento, comandos e recálculo

- Cada operação produz um corpo novo (`Feature.output`) e oculta as entradas. `Command::apply` roda numa cópia candidata do documento (`PreparedCommand`), em geral na thread de trabalho; uma falha descarta a cópia inteira.
- Mudar parâmetro, rascunho ou posição de corpo recalcula todo o histórico da peça (`features::rebuild`).
- O identificador do corpo de saída é reservado antes da construção (`allocate_body_id`), porque os nomes das faces levam o dono.
- Malha é apresentação/intercâmbio, nunca a geometria precisa. A exportação STL de sólidos re-tessela o B-rep com a qualidade escolhida.

### Interface

- `desktop/project_editor.rs` coordena trabalhos em segundo plano (`self.start(...)` → `Outcome`), com proteção contra resultados de outra aba ou revisão. Cálculos com o núcleo (recálculo, medição, exportação) não rodam na thread da interface.
- O redesenho é sob demanda. A cena é uma malha única por revisão; o upload à GPU ocorre na interface.
- Não escreva textos visíveis direto nos widgets: acrescente uma variante em `i18n::Text`.
- Não há como testar a janela interativamente nesta sessão; use `--render-project-preview` e `--render-preview` (este confere também a triangulação) para capturas reais.

### Convenções de E/S

- Exportações e diretórios de ensaio nunca sobrescrevem arquivos: destinos novos, ou sufixos em `write_stl_files`.
- Limites explícitos: STL até 128 MiB e 2 milhões de triângulos; STEP até 256 MiB; B-rep até 64 MiB por corpo.
- Unidades internas em mm e radianos; expressões aceitam unidades (`"espessura + 1 mm"`, `"118 deg"`).

## CI

`.github/workflows/ci.yml` roda fmt, clippy, testes e ensaios de linha de comando em Ubuntu 24.04 e Windows 2025, e jobs `occt-linux` (apt, OCCT 7.6) e `occt-windows` (conda-forge) com testes `--features occt`, `--validate` e `--validate-part`. O repositório público é `trocolis/3dnator` no GitHub; a CI roda a cada push e pull request.

## Dependências

Ao adicionar uma dependência direta, registre-a em `THIRD_PARTY.md`. O OCCT é externo (LGPL 2.1 com exceção) e não deve ser incorporado ao repositório. O `Cargo.lock` mantém `clipboard-win` 5.4.1 (yanked) por restrição de cache offline; atualize antes de distribuir para Windows. Sem rede, use `cargo update --offline -p <crate>` para registrar mudanças de dependências internas.
