# Validação local do protótipo

## Incremento 0.2.0 — 26/10/2026

Validação em Linux x86_64 com as dependências do cache, sem SDK OCCT ou servidor gráfico acessível.

| Verificação | Resultado |
| --- | --- |
| Formatação e Clippy do workspace, com avisos negados | Aprovados |
| Testes do workspace | 22 aprovados |
| Ciclo CLI de projeto com dois corpos | Aprovado, incluindo reabertura após remover o STL original |
| Posições de corpo/instância | Preservadas, sem alterar vértices locais |
| Salvamento, desfazer/refazer e recuperação separada | Aprovados nos ensaios e testes de persistência |
| Limpeza de recuperação | Projetos normais preservados mesmo na mesma pasta |
| Resultado antigo de outra aba/revisão | Rejeitado no teste do controlador |
| Interface real em egui/wgpu, sem janela | Capturada em 1280 × 820, sem erro de validação wgpu |
| Diálogos nativos e interação em janela | Pendentes de ambiente gráfico |
| Windows e integração OCCT | Pendentes de execução; workflows preparados |

Evidências locais, ignoradas pelo Git:

- `target/project-validation/run-003/report.json`: oito verificações do ciclo de projeto, resultado `passed`.
- `target/project-validation/run-003/support.3dnator`: uma peça com dois corpos; a instância foi deslocada 100 mm em X e o segundo corpo 30 mm em Y dentro da peça.
- `target/project-validation/run-003/recovery.3dnator`: snapshot posterior de recuperação, separado do salvamento manual.
- `target/project-validation/editor-0.2-final.png`: tela produzida pela composição real do aplicativo, carregando o projeto e mostrando uma aba auxiliar.

O adaptador da captura foi llvmpipe, via backend OpenGL do wgpu. Não foi realizado benchmark da máquina de referência de 8 GB/GPU integrada. A etapa de edição paramétrica ainda não foi implementada.

Os testes novos também cobrem transformação rígida, amplificação de vértices por instâncias, limites do documento, compartilhamento de geometria entre snapshots, falhas atômicas e marcação correta de um salvamento que termina após outra edição.

## Registro histórico do protótipo 0.1.0

Registro de 25/10/2026. O teste local usa Linux x86_64, Rust 1.99.1 e as dependências disponíveis no cache, sem acesso de rede pelo terminal.

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --all -- --check` | Aprovada |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Aprovada sem avisos |
| `cargo test --workspace --offline --locked` | 9 testes aprovados |
| `cargo build -p nator-prototype --offline --locked` | Aprovada para Linux |
| Exportar e reimportar suporte em STL | 60 × 20 × 30 mm, volume 10.080 mm³, 20 triângulos, sem arestas abertas ou inconsistentes |
| Captura headless pelo wgpu | Aprovada; superfícies e sobreposição de triangulação exercitadas |
| Checagem de tipos e Clippy do caminho Rust com OCCT | Aprovada por emissão de metadados, incluindo o código desktop; não compila nem vincula C++/OCCT |
| Compilação com `--features occt` | Bloqueada: SDK e `OpenCASCADEConfig.cmake` ausentes |
| `--validate` sem OCCT | Falha explícita com saída 1, sem produzir relatório de sucesso |
| Abrir janela Wayland/X11 | Bloqueada: compositor e servidor X inacessíveis neste ambiente |
| Compilação e execução Windows | Pendente; workflow preparado e não executado |

Os testes cobrem volume analítico, validação de índices/triângulos, bordas abertas e orientação, zoom ancorado, STL ASCII/binário, escala, truncamento, contagens inválidas, valores não finitos, preservação de arquivos existentes, remoção de saídas incompletas e perda de detalhes na quantização f32.

## Evidência gráfica e consumo preliminar

Captura real produzida em `target/prototype/preview-solid.png`; relatório em `target/prototype/render-report.json`. Esses arquivos são artefatos locais ignorados pelo Git.

- Adaptador: llvmpipe, LLVM 23.1.3, renderização por software.
- Backend: OpenGL, através do wgpu.
- Captura: 960 × 640 pixels, com 41.314 pixels de superfície identificados.
- Tempo informado pelo comando: aproximadamente 224 ms, incluindo inicialização, renderização, leitura e gravação PNG.
- Pico RSS do processo no Linux: 169.508.864 bytes, aproximadamente 162 MiB, medido com `resource.getrusage` no processo que o iniciou.

Esses valores não representam fluidez interativa nem validam a meta de computador com 8 GB/GPU integrada. O teste usou uma malha pequena, build de desenvolvimento e renderização por software. A validação completa depende de SDK OCCT, Windows e hardware de referência.

## Pendências do ambiente

- Sem resolução DNS pelo terminal: não foi possível obter o SDK OCCT ou atualizar dependências fora do cache.
- A pasta `.git` está protegida contra escrita e não contém um repositório inicializado. Não foi possível criar commits; nenhum remoto foi publicado.
- Consulte [PROTOTYPE.md](PROTOTYPE.md) para os requisitos e comandos reproduzíveis.
