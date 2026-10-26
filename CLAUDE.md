# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Contexto

3dnator é um CAD paramétrico em Rust (licença MIT). O estado atual é um **protótipo 0.2.0** com documento, comandos, abas, recuperação e persistência `.3dnator` para corpos de malha. O editor paramétrico e os rascunhos ainda **não existem**. README.md e ROADMAP.md descrevem o escopo aprovado, não funcionalidades entregues; não presuma que algo descrito ali já está implementado.

As convenções obrigatórias (idioma, arquitetura, erros, versões, commits) estão em `AGENTS.md`; siga-as. Em resumo: identificadores em inglês, comentários e documentação em português, textos de interface em português via catálogo de tradução, CHANGELOG.md atualizado a cada mudança, nada de push sem autorização.

## Comandos

Requer Rust 1.95+ (edition 2024). Use `--locked`, como na CI.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# Um crate ou um teste específico
cargo test -p nator-geometry --locked
cargo test -p nator-project-io --locked rejects_truncation

# Visualizador desktop (egui/eframe + wgpu)
cargo run -p nator-prototype

# CLI sem janela (sem a feature desktop)
cargo run -p nator-prototype --no-default-features -- --export-reference reference.stl
cargo run -p nator-prototype --no-default-features -- --inspect reference.stl
cargo run -p nator-prototype -- --render-preview target/prototype/preview.png   # captura wgpu offscreen; destino deve ser novo

# Núcleo CAD real (exige SDK OCCT 7.x + CMake instalados)
cargo clippy -p nator-prototype --no-default-features --features occt --locked -- -D warnings
cargo run -p nator-prototype --no-default-features --features occt -- --validate validation-output/run-001   # diretório deve ser novo
```

Para OCCT, configure `OpenCASCADE_DIR` (pasta com `OpenCASCADEConfig.cmake`) ou `CMAKE_PREFIX_PATH`; as bibliotecas compartilhadas precisam estar no carregador (no Windows, DLLs no PATH). Receitas de instalação para Ubuntu e Windows/conda-forge estão em `docs/PROTOTYPE.md` e `.github/workflows/ci.yml`. No Linux, o build com janela precisa de `libxkbcommon-dev` e `libwayland-dev`.

## Lints do workspace

Definidos em `Cargo.toml` e herdados por todos os crates (`[lints] workspace = true`):

- `unsafe_code = "deny"`: a única exceção é `crates/kernel/src/native.rs` (`#[allow(unsafe_code)]`). Todo bloco `unsafe` leva comentário `// SAFETY:`.
- Clippy nega `unwrap_used`, `expect_used` e `panic`, inclusive em testes; os testes retornam `Result<(), Box<dyn Error>>` e usam `?`.

## Arquitetura

`ARCHITECTURE.md` define nove módulos planejados e seus limites de dependência. Hoje existem apenas:

| Crate | Papel atual |
| --- | --- |
| `crates/geometry` (`nator-geometry`) | Tipos base sem dependências: `Point3 = [f64; 3]` em mm, `Mesh` validada, `Topology`, `reference_bracket()` |
| `crates/kernel` (`nator-kernel`) | Fronteira do núcleo CAD; com a feature `occt`, expõe `Solid` sobre uma ponte C++ |
| `crates/project_io` (`nator-project-io`) | STL e pacote `.3dnator` versão 1, com salvamento atômico e recuperação |
| `crates/document` (`nator-document`) | Peças, corpos de malha, instâncias e transformações |
| `crates/commands` (`nator-commands`) | Transações e desfazer/refazer, com snapshots que compartilham malhas |
| `crates/renderer` (`nator-renderer`) | Câmera ortográfica e `Viewport` wgpu que renderiza para textura offscreen |
| `apps/prototype` (`nator-prototype`) | Binário: CLI, visualizador eframe (`desktop.rs`), catálogo i18n, captura (`preview.rs`) e validação OCCT (`validation.rs`) |

Ainda não há crates `sketch`, `ui` e `app`; por enquanto a interface vive em `apps/prototype`. Ao criá-los, respeite a direção de dependências do ARCHITECTURE.md (`geometry` na base; `sketch`/`kernel` sem dependência de `document`/`commands`/`ui`/`app`; regras de negócio sem egui/eframe/wgpu).

### Integração OCCT (pontos delicados)

- Camadas: `bridge.cpp` (único código que conhece tipos OCCT, ABI C `nator_*` com erro em `thread_local`) → `native.rs` (único módulo Rust com FFI) → `Solid` (API pública). Nenhum tipo OCCT ou ponteiro cru sai do crate `kernel`; erros nativos viram `KernelError`.
- `build.rs` só faz algo com a feature `occt`: chama CMake em `native/`, compila a ponte estática e lê `nator-links.txt`, gerado pelo CMake, para vincular as bibliotecas OCCT. Compilação cruzada é rejeitada; para validar Windows, use um runner Windows.
- `CMakeLists.txt` exige OCCT 7.x (alvo: 7.6 a 7.9) e lida com a reorganização dos toolkits STEP no 7.8 (`TKDESTEP` contra `TKSTEP*`).
- `Solid` é `!Send`/`!Sync` de propósito (`PhantomData<Rc<()>>`): objetos nativos ficam na thread que os criou. Operações (`cut`, `fillet`, `chamfer`) devolvem um sólido novo e preservam o original.
- Índices de aresta passados a `fillet`/`chamfer` são temporários e **não** servem como identificação persistente para o histórico paramétrico.
- Sem a feature `occt`, operações CAD falham explicitamente via `nator_kernel::require_available()`. Não crie núcleo alternativo nem simule resultados.

### Malha versus sólido

`Mesh` é representação de apresentação/intercâmbio, nunca a geometria precisa. `Mesh::new` rejeita índices inválidos, coordenadas não finitas e triângulos degenerados; os campos são privados para manter essa invariante. `topology()` reconhece vértices por igualdade exata de bits, sem solda por tolerância, e só verifica arestas localmente (não detecta auto-interseções). Malhas STL abertas são aceitas como referência, sem reparo nem conversão para sólido.

### Interface e trabalho em segundo plano

- `desktop/project_editor.rs` coordena E/S e preparação de cena em thread de trabalho e recebe o resultado por `mpsc`; o redesenho é sob demanda, sem ciclo de animação contínuo. A preparação CPU dos buffers ocorre na thread de trabalho; o upload à GPU continua na thread da interface.
- O renderer desenha em textura própria, registrada no egui como textura nativa; `--render-preview` usa o mesmo caminho sem abrir janela.
- Textos de interface ficam em `apps/prototype/src/i18n.rs` (enum `Text` + `text()`); não escreva strings visíveis diretamente nos widgets.

### Convenções de E/S observadas no código

- Exportações e diretórios de validação nunca sobrescrevem arquivos existentes; o destino deve ser novo.
- Entradas importadas têm limites explícitos: STL até 128 MiB e 2 milhões de triângulos; STEP até 256 MiB. A escala do STL para mm é aplicada na leitura.
- Unidades internas em milímetros e radianos (ex.: `tessellate(linear_mm, angular_rad)`).

## Formato de projeto e ensaios atuais

O pacote ZIP `.3dnator` usa manifesto JSON e malhas binárias f64, conforme `docs/PROJECT_FORMAT.md`. Ainda não preserva sólidos CAD, rascunhos ou histórico paramétrico. Recuperação usa `NATOR_RECOVERY_DIR` quando definido; configure um diretório temporário nos testes que executam esse fluxo.

```sh
cargo run -p nator-prototype --no-default-features -- --validate-project target/project-check/run-001
cargo run -p nator-prototype --no-default-features -- --inspect-project target/project-check/run-001/support.3dnator
cargo run -p nator-prototype -- --render-project-preview target/project-check/run-001/support.3dnator target/project-check/editor.png
```

## CI

`.github/workflows/ci.yml` roda fmt, clippy, testes e a CLI de STL em Ubuntu 24.04 e Windows 2025, além de jobs separados `occt-linux` (apt) e `occt-windows` (conda-forge), que publicam `validation-output/` como artefato. Mudanças em dependências nativas precisam funcionar nos dois sistemas.

## Dependências

Ao adicionar uma dependência direta, registre-a em `THIRD_PARTY.md`. O OCCT é externo (LGPL 2.1 com exceção) e não deve ser incorporado ao repositório. O `Cargo.lock` mantém `clipboard-win` 5.4.1 (yanked) por restrição de cache offline; ele precisa ser atualizado antes de distribuir para Windows.
