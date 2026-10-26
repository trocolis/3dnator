# Validação local do protótipo

## Cubo de orientação e giros da vista — 26/10/2026

- Reconstruído o cubo como poliedro fechado: 26 regiões, 48 arestas compartilhadas exatamente por duas faces e sem invasão dos semiespaços vizinhos. Os cliques escolhem polígonos visíveis, incluindo arestas/cantos, em vez de retângulos sobrepostos. Removido o indicador separado de linhas XYZ.
- Tesselação exercitada em **832 combinações** de azimute, elevação e giros de 90°, incluindo polos e orientações quase coplanares. Todos os vértices permaneceram finitos e dentro dos limites do navegador. O preenchimento usa triângulos, os contornos são segmentos independentes e o controle tem recorte próprio; faces degeneradas são descartadas.
- Eventos egui verificam clique nas faces em ambas as projeções, arraste sem selecionar/mover o modelo e sem encaixe indevido ao soltar, setas percorrendo 0°/90°/180°/270° e retorno a 0° pelo indicador. A câmera cruza os polos com base contínua e ortonormal; seleção, matriz da GPU, pan e zoom no cursor permanecem coerentes após inclinar a tela.
- **105 testes sem OCCT e 137 com OCCT 7.9.3 aprovados** no workspace, com formatação e Clippy sem avisos nas duas configurações. Os 27 testes da aplicação/renderizador foram repetidos após o acabamento dos rótulos e da pintura.
- Compilação desktop Linux e capturas reais egui/wgpu aprovadas em 1280 × 820 e 900 × 600, com imagens inspecionadas: `target/codex-validation/view-cube-20261026/navigator.png` e `navigator-compact.png`. Prévias ortográfica/perspectiva também aprovadas, incluindo cores, arestas, triangulação e transparência, sem erros de validação wgpu/GL llvmpipe. Não foi declarada execução Windows ou interação com janela nativa.

## Controles integrados da janela — 26/10/2026

- Configurada janela sem decoração nativa, com minimizar/maximizar/restaurar/fechar na barra escura superior, conforme esclarecimento do usuário. Capturas reais em 1280 × 820 e 900 × 600 inspecionadas: `target/codex-validation/window-frame-20261026/model-aligned.png` e `compact-aligned.png`, sem erros de validação wgpu/GL llvmpipe.
- Quatro regressões de eventos verificam os comandos de minimizar/maximizar/restaurar; o encaminhamento de fechar ao editor sem forçar saída nativa; arraste/duplo clique no fundo da barra; redimensionamento das bordas e sua desativação ao maximizar. O ensaio detectou e corrigiu a disputa entre o arraste do fundo e o clique dos botões.
- Nove testes da aplicação aprovados com e sem OCCT, incluindo as regressões anteriores da interface. Formatação e Clippy do workspace com todos os alvos e avisos negados aprovados nas duas configurações; compilação desktop Linux aprovada. A execução efetiva dos comandos pelo gerenciador de janelas e a interação com diálogos nativos não são cobertas pela captura sem janela; Windows continua adiado.

## Refinamento da interface — 26/10/2026

- Referência visual: organização compacta do SolidCraft consultada no projeto local. Ícones vetoriais e implementação produzidos no 3dnator. Estrutura e uso em [INTERFACE.md](INTERFACE.md).
- `cargo fmt --all -- --check` e Clippy com todos os alvos e avisos negados aprovados, com e sem OCCT. **91 testes sem OCCT e 123 com OCCT 7.9.3** aprovados. Os testes novos verificam clique, bloqueio de botões desabilitados, rótulo acessível, ativação por teclado, busca portuguesa sem acentos e enquadramento de perfis deslocados/círculos em canvases pequenos.
- Compilação desktop aprovada no Linux. Os seis ensaios de núcleo, projeto, rascunho, montagem/folhas, primeira peça e intercâmbio STEP passaram novamente; relatórios em `target/codex-validation/interface-20261026/validate*/report.json`.
- Capturas reais do editor e rascunho em 1280 × 820 e 900 × 600, pelo egui/wgpu/GL llvmpipe, sem erros de validação. Imagens inspecionadas; a revisão no tamanho mínimo revelou e permitiu corrigir o corte do rascunho por falta de enquadramento. Evidências: `model-final.png`, `compact-final.png`, `sketch-final.png` e `sketch-fitted.png` no diretório do ensaio. Capturas são evidências locais ignoradas pelo Git.
- Prévias ortográfica e em perspectiva do renderizador também passaram, preservando cores, triangulação e transparência (razão de brilho de aproximadamente 0,698). O fundo configurável do editor não alterou o padrão usado pelos ensaios do renderizador.
- Windows foi adiado por orientação do usuário e não foi testado. O workflow mantém a validação nativa como opção manual `validate_windows`; a rotina atual prioriza Linux. Os testes de eventos egui não equivalem a um ensaio de teclado/mouse e diálogos em uma janela nativa.

## Auditoria do plano — 26/10/2026

Situação completa e critérios de aceite em [PLAN_AUDIT.md](PLAN_AUDIT.md). Repetidos `cargo fmt --all -- --check`, Clippy do workspace com todos os alvos e avisos negados, e testes com/sem `occt`: **87 sem OCCT e 119 com OCCT**, todos aprovados. Compilação desktop debug e release aprovada no Linux com OCCT 7.9.3. Não houve alteração de versão do aplicativo ou do formato de projeto.

Cinco regressões de empacotamento Linux aprovadas: destino existente preservado, arquivo comprimido preservado, dependência ausente rejeitada antes da saída, erro compreensível para opção sem valor e execução após mover o pacote e tornar o SDK indisponível. O SDK mínimo do ensaio tem uma biblioteca OCCT com dependência TBB transitiva em caminho com espaços. As duas regressões PowerShell estão preparadas na CI, ainda sem execução local.

O pacote real reuniu 26 bibliotecas OCCT pelo SONAME, sem symlinks, junto com os textos LGPL/exceção. Após movê-lo, os seis ensaios `--validate`, `--validate-project`, `--validate-sketch`, `--validate-editor`, `--validate-part` e `--validate-step` passaram em ambiente limpo. Capturas ortográfica, perspectiva e do editor passaram pelo wgpu/GL llvmpipe, sem erros de validação; o editor e a folha STEP foram inspecionados. Evidências em `target/codex-validation/audit-20261026/`.

O `.tar.gz` release também foi extraído em outro diretório e passou nos ensaios OCCT, da primeira peça e de montagem STEP com ambiente limpo. A verificação de procedência confirmou que os dois textos de licença OCCT estão idênticos, byte a byte, aos arquivos do SDK local.

### Limites numéricos provisórios para a máquina de referência

Os limites abaixo são critérios de aceite inicial, estabelecidos após as medições existentes. Não constituem desempenho comprovado em 8 GB. Usar build release, OCCT, área 3D de 1280 × 800, cinco quadros de aquecimento e 120 de medição, tesselação do suporte com desvio linear de 0,05 mm e angular de 0,25 rad. Rodar três vezes em processos novos, com GPU integrada física e a máquina ociosa; comparar a mediana das três execuções. A abertura mede a leitura do projeto, separada da preparação e do upload gráfico.

| Cenário | Abrir projeto | Quadro p95 | RSS em repouso / pico |
| --- | --- | --- | --- |
| Suporte simples (280 triângulos) | ≤ 250 ms | ≤ 16,7 ms | ≤ 256 / 384 MiB |
| 400 instâncias (112 mil triângulos) | ≤ 500 ms | ≤ 16,7 ms | ≤ 384 / 512 MiB |
| STL de 50 MB (1 milhão de triângulos) | ≤ 2.000 ms | ≤ 33,3 ms | ≤ 512 / 768 MiB |

Importação do STL e análise de topologia: cada etapa ≤ 2.000 ms. Alteração de parâmetro do suporte: p95 ≤ 250 ms, pico RSS ≤ 128 MiB no processo de recálculo. Abertura da janela até o primeiro quadro utilizável: ≤ 5 s, medidos separadamente em três lançamentos com captura de tela/cronômetro. Confirmar que teclado, mouse e cancelamento respondem durante trabalho em segundo plano. No Windows, coletar memória também por ferramenta do sistema, pois o relatório atual só implementa RSS no Linux. Uma medição ausente não aprova o critério.

### Repetição release nesta auditoria

Intel Core i5-11400H, 16 GB, Linux Arch, OCCT 7.9.3; adaptador **llvmpipe/GL (CPU)**, não a Intel UHD. Medição única para diagnóstico, sem aprovar o aceite da GPU integrada. Relatório em `target/codex-validation/audit-20261026/benchmark/report.json`:

| Cenário | Abrir projeto | Quadro médio / p95 | RSS repouso / pico |
| --- | --- | --- | --- |
| Suporte | 0,09 ms | 0,64 / 0,89 ms | 172 / 182 MiB |
| 400 instâncias | 1,32 ms | 16,89 / 17,91 ms | 210 / 254 MiB |
| STL de 1 milhão de triângulos | 364,51 ms | 86,88 / 94,40 ms | 431 / 522 MiB |

Importação STL: 270,45 ms; topologia: 306,23 ms; recálculo p95: 21,08 ms; pico do processo de recálculo: 37,5 MiB. Os quadros de instâncias e da malha excedem o objetivo numérico sob renderização por software; isso fica registrado e não é convertido em uma alegação de aprovação no hardware de referência. O buffer gráfico atual usa 96 bytes por triângulo, incluindo a cor.

## Continuação 0.3.0 — empacotamento, distribuição e licenças (26/10/2026)

- Estratégia de distribuição e empacotamento documentada em `docs/DISTRIBUTION.md`, estabelecendo a arquitetura de pacotes portáteis autocontidos e os scripts automatizados `scripts/package_linux.sh` e `scripts/package_windows.ps1`.
- Ensaio de empacotamento no Linux: o script reuniu o executável ELF, resolveu automaticamente as dependências dinâmicas via `ldd` copiando 78 bibliotecas e symlinks do OCCT 7.9.3 para `lib/`, e gerou o launcher `3dnator` com `LD_LIBRARY_PATH` local.
- Execução do pacote portátil em ambiente estritamente limpo (`env -i PATH=/usr/bin:/bin`): verificado que `./3dnator --version` identifica corretamente "3dnator 0.3.0 | OCCT: habilitado" e que o ensaio `--validate` completa com sucesso todos os passos de B-rep, corte, furo, arredondamento e intercâmbio STEP/STL, confirmando que a distribuição opera de forma independente e autocontida.
- Inventário de licenças em `THIRD_PARTY.md` enriquecido com o aviso proeminente e texto integral da *Open CASCADE Technology Exception (version 1.0)* à GNU LGPL 2.1, instruções para obtenção de fontes e garantia de substituição de bibliotecas dinâmicas pelo usuário final.

## Continuação 0.3.0 — cortes precisos do B-rep (26/10/2026)

83 testes sem OCCT e 113 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. Uma seção anular no núcleo preserva os dois contornos, a área esperada dentro da tolerância de amostragem e o volume do sólido original. Outra regressão impede que sobreposições de corpos distintos cancelem suas hachuras. Vistas e traços dos planos usam a interseção preparada uma única vez por corte; curvas de HLR e corte compartilham amostragem por intervalos C2 com desvio de 0,01 mm. O ensaio `--validate-step target/codex-validation/precise-sections-001` gerou PNG/PDF com cortes precisos A–A/B–B da montagem posicionada. A legenda identifica **HLR E CORTES DO B-REP**; contextos com malhas continuam aproximados. O formato de projeto permanece em 10.

## Continuação 0.3.0 — múltiplos cortes e formato da folha (26/10/2026)

82 testes sem OCCT e 111 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. As regressões verificam manutenção de identificadores após remoção de A–A, retrato/paisagem, dimensões do PNG na resolução escolhida, rejeição de cortes repetidos ou inválidos e salvamento/reabertura/desfazer/refazer das configurações. `--validate-editor target/codex-validation/multiple-sections-001` aprovado, com folha A4 em retrato, cortes A–A/B–B perpendiculares e PNG em 96 DPI. Projeto, PNG e PDF estão no diretório do ensaio; a imagem foi inspecionada. O formato de projeto passou a 10, com cortes adicionais vazios, paisagem e 150 DPI por padrão em folhas anteriores.

## Continuação 0.3.0 — vistas precisas do B-rep (26/10/2026)

79 testes sem OCCT e 108 com OCCT aprovados; formatação e Clippy com avisos negados aprovados nas duas configurações. As regressões novas confirmam que um bloco atrás de outro recebe linhas ocultas, que o perímetro visível externo confere com o analítico e que a silhueta circular é extraída do B-rep com controle explícito de desvio. A folha de um cilindro com malha propositalmente grosseira usa HLR; acrescentar um corpo de malha troca para o método aproximado identificado na legenda. O ensaio `--validate-step target/codex-validation/step-hlr-001` também gera `drawing.pdf` e `drawing.png`, com quatro vistas HLR da montagem, escala 0,5 e linhas ocultas. A imagem foi inspecionada. Cortes ainda são calculados sobre a malha e isso permanece explícito na legenda.

## Continuação 0.3.0 — montagem STEP (26/10/2026)

- 79 testes sem OCCT e 105 com OCCT 7.9.3 aprovados. Três regressões novas verificam nomes UTF-8, definições compartilhadas por raízes e subpeças, cores de corpo e ocorrência, posições rotacionadas, persistência sem o STEP de origem, exportação/reimportação, atomicidade e rejeição de ciclos e malhas no escopo de exportação.
- `--validate-step target/codex-validation/step-001` aprovado: bloco de 6.000 mm³, duas ocorrências com cores distintas e montagem posicionada. O arquivo de origem foi removido antes da reabertura e da segunda exportação. Projeto resultante em `imported.3dnator`, STEP em `assembly.step` e relatório em `report.json`.
- A ponte XCAF foi compilada localmente com o SDK 7.9.3. A CI Linux/Windows recebeu o ensaio; execução com OCCT 7.6 e no Windows permanece pendente. Superfícies abertas, referências externas e cores por face não foram declaradas como suportadas.

## Continuação 0.3.0 — cores sólidas (26/10/2026)

79 testes sem OCCT e 102 com OCCT aprovados. Formatação e Clippy com avisos negados aprovados nas duas configurações. A regressão nova verifica comando de cor, cena, clonagem, salvamento/reabertura e desfazer/refazer. A captura `target/codex-validation/colors-001.png` verificou por leitura real de pixels 31.727 pixels da cor de teste, além de triangulação e transparência. Formato de projeto 8; projetos antigos permanecem com a cor neutra. O buffer passa de 84 para 96 bytes por triângulo; medições históricas de memória anteriores a essa mudança não foram repetidas.

## Continuação 0.3.0 — espelhamento de conjuntos (26/10/2026)

- Testes do workspace: 78 aprovados sem OCCT e 101 com OCCT 7.9.3. Formatação e Clippy com avisos negados aprovados nas duas configurações.
- Novas regressões cobrem planos oblíquos nas coordenadas de um pai rotacionado, corpos posicionados localmente, definições compartilhadas por vários caminhos, grupos internos, ocorrências e corpos finais ocultos, salvamento/reabertura, duplo espelhamento, desfazer/refazer e preservação do documento e do refazer após falhas.
- A rotação da imagem mantém determinante +1. Casos em ±90° verificam também a correção da conversão de matrizes para ângulos, que perdia precisão perto da singularidade.
- Com o núcleo real, verificados volume, área e centro de massa de três ocorrências sólidas em um conjunto com corpo de malha. Alterar a espessura da subpeça espelhada de 8 para 15 mm depois de reabrir recalcula seus corpos visível e oculto, mantendo as definições originais independentes.
- Ensaios `--validate-editor` e `--validate-part` aprovados; relatórios e arquivos em `target/codex-validation/assembly-editor-001/` e `assembly-part-001/`, respectivamente. O ciclo da peça conserva volume analítico, referências, persistência e intercâmbio STEP/STL.
- Validação Windows e interação manual com o diálogo de montagem continuam pendentes. O formato do projeto permanece na versão interna 7.

## Continuação 0.3.0 — perspectiva (26/10/2026)

- `cargo fmt --all -- --check` e Clippy do workspace com avisos negados, com e sem OCCT: aprovados.
- Testes do workspace: 73 aprovados sem OCCT e 95 com OCCT 7.9.3. A câmera é verificada nas duas projeções quanto a zoom no cursor, alteração dos limites da cena, correspondência entre seleção/anotações e matriz da GPU, profundidade wgpu [0, 1], enquadramento e movimento livre.
- Capturas reais pelo wgpu, backend GL/llvmpipe: perspectiva com 31.763 pixels de superfície e 1.564 pixels alterados pela triangulação; ortográfica com 39.690 e 1.644, respectivamente. O estilo transparente reduziu o brilho para aproximadamente 69,8% nas duas projeções, sem erro de validação wgpu.
- Evidências locais: `target/codex-validation/perspective-001.png`, `orthographic-001.png` e `editor-001.png`. As capturas não validam interação manual ou desempenho de GPU física.

## Incremento 0.3.0 com OCCT — 26/10/2026

Validação em Linux x86_64 (Arch, Rust 1.99.1, GCC 16.2.1) com OCCT 7.9.3 extraído do pacote oficial do Arch, checksum conferido com o banco de dados assinado do pacman. Primeira execução da ponte C++ com um SDK real.

| Verificação | Resultado |
| --- | --- |
| Configuração CMake da ponte | Falhava em qualquer SDK 7.x pela ordem dos componentes; corrigida |
| `cargo clippy --workspace --all-targets` com e sem `occt` | Aprovado, avisos negados |
| Testes do workspace com `occt` | 90 aprovados (68 sem `occt`): nomeação e operações mecânicas no núcleo; fluxo paramétrico, furos, padrões, espelhamento, divisão, montagem espelhada, medição exata e qualidade de exportação nos comandos |
| `--validate` (primitivas, corte, furo, arredondamento, chanfro, erro preservando a origem, tesselação, STEP/STL) | 9 passos aprovados; 37 ms; pico RSS 46 MiB |
| `--validate-project`, `--validate-sketch`, `--validate-editor` com o binário `occt` | Aprovados |
| `--validate-part` (critério de entrega da etapa 2) | Aprovado: volume final 14.375,365 mm³ igual ao analítico; STEP reimportado com o mesmo volume e caixa 60 × 25 × 30 mm; STL fechado e orientado, 656 triângulos, desvio de volume de 0,001% |
| Janela real em Wayland (GPU Intel UHD/NVIDIA) | Abre e permanece estável por 20 s; interação não testada |
| Windows, Ubuntu (OCCT 7.6) e diálogos nativos | Pendentes; workflows preparados |

### Medições de leveza (`--benchmark`, release)

Notebook com Intel Core i5-11400H, 16 GB de RAM, GPU integrada Intel UHD (Tiger Lake GT1, Mesa 26.2.5, Vulkan) e GPU dedicada GeForce GTX 1650. A integrada é a referência do ROADMAP e é escolhida por padrão. Cada cenário roda em processo próprio, com a área 3D em 1280 × 800 e 120 quadros de órbita contínua; o tempo de quadro inclui a espera pela GPU.

| Cenário | Triângulos | Abrir projeto | Quadro médio / p95 (integrada) | Quadro médio (dedicada) | RSS em repouso / pico (integrada) |
| --- | --- | --- | --- | --- | --- |
| Peça simples (suporte OCCT com furo e arredondamento) | 280 | 0,1 ms | 0,66 / 0,74 ms | 0,06 ms | 146 / 148 MiB |
| 400 instâncias da peça | 112 mil | 1,0 ms | 3,71 / 5,01 ms | 0,15 ms | 151 / 159 MiB |
| Malha STL importada (toro, 50 MB) | 1 milhão | 330 ms | 8,56 / 9,15 ms | 1,43 ms | 232 / 312 MiB |

- Importar o STL de 1 milhão de triângulos leva 268 ms, e a análise de topologia e arestas vivas, 272 ms. O projeto resultante tem 6,5 MB.
- Recalcular o histórico do suporte (rascunho, extrusão, furo, corte e arredondamento) leva 20,7 ms por mudança de parâmetro; a construção inicial, 27 ms.
- Cerca de 145 MiB do RSS em repouso são o processo com o driver Vulkan e os buffers da GPU integrada, que usa a memória do sistema.

Antes das otimizações deste incremento, a mesma malha de 1 milhão de triângulos tinha pico de 575 MiB, abria em 507 ms e levava 211 ms para preparar os dados da GPU. As mudanças foram:

- remover o buffer de linhas da triangulação: as linhas agora são desenhadas no shader por coordenadas baricêntricas, o que eliminou 144 bytes por triângulo;
- fundir vértices idênticos na importação STL e na tesselação: a malha passou de 72 para cerca de 12 bytes por triângulo na CPU e no arquivo;
- calcular as arestas vivas uma vez por geometria, em vez de a cada exibição;
- gravar os vértices da GPU sem uma alocação por vértice.

Esses números valem para uma máquina mais forte que a referência de 8 GB com GPU integrada. A medição na própria máquina de referência ainda está pendente.

O cenário clássico de nomeação topológica foi verificado: o arredondamento do canto interno do suporte permanece na mesma aresta quando a espessura muda de 6 para 9 mm, e o volume confere com (1 − π/4)·r²·L. Substituir o canto por uma diagonal faz o recálculo falhar com "Referência perdida", sem aplicar o arredondamento em outra aresta e sem alterar o documento.

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

## Pendências do ambiente no registro inicial (histórico)

- Sem resolução DNS pelo terminal: não foi possível obter o SDK OCCT ou atualizar dependências fora do cache.
- A pasta `.git` está protegida contra escrita e não contém um repositório inicializado. Não foi possível criar commits; nenhum remoto foi publicado.
- Consulte [PROTOTYPE.md](PROTOTYPE.md) para os requisitos e comandos reproduzíveis.
