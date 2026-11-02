# Instruções para agentes — 3dnator

CAD paramétrico em Rust. Protótipo 0.9.0, workspace com edition 2024 e Rust 1.95+.
Este arquivo reúne o que um agente precisa saber antes de editar. Contratos detalhados ficam em
`ARCHITECTURE.md` e `docs/`; `CLAUDE.md` tem um guia mais longo, mas cita versões antigas — confirme
números no código.

## Comandos

Sempre use `--locked`, como a CI. A ordem de verificação é fmt → clippy → testes.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# Um crate ou um teste específico
cargo test -p nator-sketch --locked
cargo test -p nator-project-io --locked rejects_truncation

# Editor (egui/eframe + wgpu); sem `occt`, não constrói sólidos
cargo run -p nator-prototype --features occt --locked
```

Os ensaios de linha de comando não precisam do desktop; use `--no-default-features`. Todo diretório
de saída deve ser novo, pois nada é sobrescrito.

```sh
cargo run -p nator-prototype --no-default-features --locked -- --validate-project target/check/project
cargo run -p nator-prototype --no-default-features --locked -- --validate-sketch target/check/sketch
cargo run -p nator-prototype --locked -- --validate-editor target/check/editor
cargo run -p nator-prototype --locked -- --render-project-preview PROJETO.3dnator target/check/editor.png
```

`--validate-editor` e os comandos `--render-*` exigem a feature `desktop` (padrão). Para capturar
estados da interface sem servidor gráfico, veja `--render-ui-scenario` em `--help` e
`docs/INTERFACE.md`.

## OCCT

A feature `occt` compila `crates/kernel/native/bridge.cpp` via CMake e vincula OCCT 7.6–7.9. Configure
`OpenCASCADE_DIR` (pasta com `OpenCASCADEConfig.cmake`) ou `CMAKE_PREFIX_PATH`; no Linux deixe as
bibliotecas em `LD_LIBRARY_PATH`, no Windows as DLLs no `PATH`. Compilação cruzada é rejeitada.

Nesta máquina existe um SDK extraído em `target/claude-occt-sdk/root/usr`. Use um
`CARGO_TARGET_DIR` separado para não invalidar o cache dos builds sem OCCT; um `cargo clean` no
target padrão apaga o SDK.

```sh
SDK=$PWD/target/claude-occt-sdk/root/usr
export OpenCASCADE_DIR=$SDK/lib/cmake/opencascade LD_LIBRARY_PATH=$SDK/lib CARGO_TARGET_DIR=$PWD/target/claude-occt
cargo clippy --workspace --all-targets --features occt --locked -- -D warnings
cargo test --workspace --features occt --locked
cargo run -p nator-prototype --features occt --locked -- --validate target/check/occt
cargo run -p nator-prototype --features occt --locked -- --validate-part target/check/part
```

Sem a feature e o SDK, operações CAD falham explicitamente via `nator_kernel::require_available()`.
Nunca crie núcleo alternativo nem simule resultados. Os testes `*_occt.rs` só rodam com a feature.

## Lints e testes

- `unsafe_code = "deny"` no workspace; a única exceção é `crates/kernel/src/native.rs`. Todo bloco
  `unsafe` leva comentário `// SAFETY:`.
- Clippy nega `unwrap_used`, `expect_used` e `panic`, inclusive em testes. Testes retornam `Result`
  e usam `?`.
- Código usado só com `occt` deve compilar sem avisos também sem a feature (veja
  `cfg_attr(not(feature = "occt"), allow(dead_code))` em `crates/kernel/src/lib.rs`).

## Arquitetura e limites

| Crate | Papel |
| --- | --- |
| `geometry` | `Point3` (mm), `Mesh` validada, topologia e `Placement` (rotação X→Y→Z) |
| `sketch` | Rascunhos, solver, ferramentas e regiões estáveis (`profiles`) |
| `kernel` | Fronteira do OCCT: `Solid`, nomes de faces, furos, B-rep |
| `document` | Peças, corpos, instâncias, grupos, parâmetros, histórico, seleção |
| `commands` | Comandos atômicos, desfazer/refazer por snapshots, recálculo, medição/exportação |
| `project_io` | Formato `.3dnator`, STL, salvamento atômico e recuperação |
| `renderer` | Câmera ortográfica e viewport wgpu |
| `drawing` | Folhas técnicas, vistas, cortes, PNG/PDF |
| `apps/prototype` | Binário: CLI de ensaios, editor eframe e i18n |

- Regras de negócio não dependem de egui, eframe ou wgpu. `geometry` é independente; `sketch` e
  `kernel` não dependem de `document`, `commands` ou da interface. Nenhum tipo OCCT sai de `kernel`.
- `Solid` é `!Send`/`!Sync`: objetos nativos ficam na thread que os criou. Para cruzar threads, use
  bytes B-rep + nomes de faces.
- Alterações do documento passam por `Command::apply`, que trabalha numa cópia candidata
  (`PreparedCommand`); uma falha descarta a cópia inteira. Mudar parâmetro, rascunho ou posição
  recalcula o histórico da peça (`features::rebuild`).
- Referências são estáveis por nomes/IDs de entidades, não por índices. `RegionRef::Index`,
  `EdgeRef::Index` e `FaceRef::Index` existem só para projetos antigos e são normalizados por
  `commands::features::normalize`. Referência perdida ou ambígua deve falhar, nunca apontar para
  outra geometria em silêncio.
- Malha é apresentação/intercâmbio; medição e exportação STL usam o B-rep preciso. A exportação STL
  de sólidos re-tessela o B-rep.
- Unidades internas: milímetros e radianos. Converta apenas na interface; expressões aceitam
  unidades (`"espessura + 1 mm"`, `"118 deg"`).

## Interface

- Não escreva textos visíveis direto nos widgets: acrescente uma variante em `i18n::Text`
  (`apps/prototype/src/i18n.rs`). Identificadores de código em inglês; comentários, documentação e UI
  em português.
- Cálculos demorados (recálculo, medição, exportação, cena) não rodam na thread da interface. Use
  `self.start(...) -> Outcome` e descarte resultados de outra aba/revisão.
- No ambiente automatizado, a janela não pode ser testada interativamente; use
  `--render-project-preview`, `--render-sketch-preview` e `--render-ui-scenario` para capturas reais.

## Dados e E/S

- O formato `.3dnator` está na versão interna **35** (`FORMAT_VERSION` em
  `crates/project_io/src/project.rs`), independente da versão do aplicativo; o leitor aceita 1–35.
- Exportações e ensaios nunca sobrescrevem: destinos novos ou sufixos em `write_stl_files`.
- Limites: STL até 128 MiB e 2 milhões de triângulos; STEP até 256 MiB; B-rep até 64 MiB por corpo.
- Ao adicionar dependência direta, registre em `THIRD_PARTY.md`. O OCCT é externo (LGPL 2.1 com
  exceção) e não deve ser incorporado ao repositório.

## CI e documentação

- `.github/workflows/ci.yml` roda fmt, clippy, testes e ensaios CLI em Ubuntu; `occt-linux` roda
  sempre, enquanto `occt-windows` só roda por `workflow_dispatch` com `validate_windows=true`.
  Linux é a prioridade atual; macOS está fora do escopo.
- `prerelease.yml` é manual e não publica nem altera tags. Os scripts em `scripts/` recusam destinos
  existentes; `python3 scripts/test_package_linux.py` verifica preservação e relocação.
- Consulte `ARCHITECTURE.md`, `docs/PROTOTYPE.md` (CLI/OCCT), `docs/PROJECT_FORMAT.md` (formato 35),
  `docs/INTERFACE.md`, `docs/VALIDATION.md` e `ROADMAP.md`.

## Git e versões

- Escreva commits pequenos e descritivos em português, após verificar a alteração. Confira
  `git status` antes de editar ou commitar: o diretório pode conter trabalho não commitado.
- Atualize `CHANGELOG.md` a cada mudança. Versões X.Y.Z: X para grandes releases, Y para
  funcionalidades, Z para correções. Tags `vX.Y.Z` são releases deliberadas.
- Não faça push nem publique releases sem autorização. Preserve licenças e atribuições de terceiros;
  o código próprio usa MIT (`LICENSE`).
