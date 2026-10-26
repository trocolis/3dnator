# Protótipo 0.3.0 — CAD paramétrico

Este incremento oferece rascunhos com restrições e parâmetros, histórico paramétrico, montagem, folhas técnicas e exportação STEP/STL. Operações de sólidos exigem a feature `occt` e o SDK externo. O histórico paramétrico é salvo no projeto; desfazer/refazer registra as edições da sessão. Pendências conhecidas estão em [ROADMAP.md](../ROADMAP.md).

## Abrir a aplicação

Requer Rust 1.95 ou superior, compilador C/C++ para dependências e bibliotecas de janela/driver gráfico. A validação local usa Rust 1.99.1 em Linux x86_64.

```sh
cargo run -p nator-prototype --features occt --locked
```

A janela começa com um projeto vazio. Use **Inserir suporte em L** ou **Importar STL** para criar uma peça com um corpo. Na árvore, selecione a peça/corpo e use **Construir nesta peça**: importações seguintes acrescentam corpos à definição ativa. **Voltar à montagem** faz a próxima importação criar outra peça.

A árvore permite selecionar, mostrar/ocultar peças e corpos. As propriedades oferecem cor sólida, renomeação, posicionamento X/Y/Z em milímetros, rotação em graus e exclusão de corpos. Posições de corpos são locais à peça; posições de instâncias são relativas à montagem. Corpos só podem ser editados na peça ativa em construção; instâncias são posicionadas na montagem. A área 3D permite selecionar geometria, medir e capturar referências de faces e arestas para operações.

Projetos abrem em abas. Cada aba mantém documento, seleção, câmera, contexto e sequência de desfazer/refazer próprios. Há um único conjunto de buffers GPU para a aba ativa. A representação da cena ainda combina malhas das instâncias, sem instanciamento gráfico otimizado.

## Navegação e comandos

- Scroll aproxima/afasta em direção ao cursor.
- Botão do meio arrasta a vista; Shift com botão do meio orbita.
- Botões oferecem vistas frontal, superior, lateral e isométrica, além de enquadramento.
- O seletor **Ortográfica / Perspectiva** muda a projeção da aba. A perspectiva tem campo vertical de 45°; o zoom e o movimento livre usam o plano do alvo. **Enquadrar** recentraliza o modelo. A vista ortográfica é o padrão.
- As sombras opcionais (desligadas por padrão) podem ser ativadas pela opção **Sombras**, aplicando sombreamento direcional nas faces opostas e projeção suave de contato no plano de apoio.
- O cubo de navegação possui faces, arestas e cantos clicáveis; X/Y/Z acompanham a câmera.
- Ctrl+S salva; Ctrl+Z desfaz; Ctrl+Shift+Z ou Ctrl+Y refazem. Campos de texto conservam seu próprio desfazer.
- As operações concluídas são atômicas e a sequência guarda até 100 ações, compartilhando as malhas entre snapshots.
- Ao fechar uma aba modificada ou encerrar a aplicação, o editor oferece Salvar, Descartar ou Cancelar.

A composição da cena e a preparação CPU dos buffers ocorrem em segundo plano. A interface só faz o upload para a GPU. Enquanto uma tarefa estiver ativa, as edições ficam desabilitadas; navegação e troca de abas continuam disponíveis. Resultados de importação e cena identificam a aba e a revisão que os originaram.

## Espelhamento de conjuntos

Selecione uma instância na árvore e abra **Organizar montagem**. Escolha o plano principal e seu deslocamento, em coordenadas do pai da instância, e use **Espelhar como nova peça**. O comando cria uma cópia independente de todo o conjunto, incluindo subpeças aninhadas, corpos ocultos e grupos internos. Componentes que compartilhavam uma definição continuam compartilhando a definição nova dentro da cópia. Os grupos do nível externo conservam seus membros originais.

Sólidos recebem uma operação final de espelhamento no histórico de cada definição clonada e continuam editáveis por parâmetros; malhas são refletidas diretamente. A visibilidade dos resultados é preservada. Uma falha em qualquer subpeça cancela a operação inteira. Desfazer/refazer trata a criação do conjunto espelhado como uma única ação.

## Salvar e recuperar

As folhas técnicas de contextos exclusivamente sólidos usam remoção de linhas ocultas no B-rep, com oclusão conjunta entre corpos. Curvas projetadas são discretizadas a 0,01 mm; a malha de exibição não determina as linhas da vista. A legenda indica **HLR DO B-REP** ou **VISTAS DERIVADAS DA MALHA** para contextos com malhas. Cortes de sólidos vêm da interseção com o B-rep, usando o mesmo controle de desvio e preservando furos internos. Corpos distintos são hachurados separadamente; cortes de contextos com malhas continuam aproximados e identificados. O cálculo preciso limita a folha a vinte mil faces e quinhentos mil segmentos projetados. Cada folha aceita A–A e até sete cortes adicionais com identificadores estáveis; a origem e a normal do plano são editáveis. O layout distribui as vistas sem alterar a escala; se não couberem, reduza a escala ou use A3. Orientação retrato/paisagem e PNG de 72 a 300 DPI são configurações da folha.

**Salvar** e **Salvar como…** usam diálogos nativos. O arquivo `.3dnator` contém definições, corpos de malha e sólidos com B-rep, nomes de faces, rascunhos, parâmetros, histórico, montagem, grupos e folhas técnicas. O arquivo importado de origem pode ser movido ou apagado após o salvamento. A versão interna é 10, independente da versão do aplicativo, com migração de projetos anteriores.

A gravação temporária é validada antes de substituir o projeto anterior. A recuperação é gravada separadamente, após dois segundos de inatividade e disponibilidade da thread de trabalho, e é oferecida ao iniciar a aplicação. Uma cópia recuperada precisa ser salva explicitamente. `NATOR_RECOVERY_DIR` permite escolher o diretório das cópias.

Detalhes e limites estão em [PROJECT_FORMAT.md](PROJECT_FORMAT.md). Miniaturas e câmera persistida ainda não fazem parte do arquivo. A câmera, incluindo a projeção, é mantida por aba durante a sessão. Desfazer/refazer começa vazio ao reabrir.

## Importar e exportar STL

A importação aceita STL ASCII e binário, com até 128 MiB e dois milhões de triângulos; a escala para milímetros é escolhida antes de importar. Malhas abertas são aceitas como referência. Vértices não finitos, triângulos degenerados e arquivos incompletos são rejeitados, sem reparo automático.

O diálogo **Exportar STL** oferece corpo, corpos da peça, cada definição uma vez, montagem em um arquivo ou todas as ocorrências. Mostra dimensões, triângulos, arquivos e tamanho antes de gravar, com qualidade rascunho/padrão/alta ou tolerâncias personalizadas. Arquivos separados usam coordenadas locais da peça; exportações conjuntas e de ocorrências usam posições de montagem. Os nomes combinam peça e corpo, com sufixos para evitar sobrescrita.

O STL binário verifica a quantização f32, fechamento local de arestas, orientação e volume positivo. Não detecta todos os defeitos de impressão, como auto-interseções ou cascas sobrepostas. Sólidos são tessellados a partir do B-rep para exportação, independentemente da malha de exibição. Malhas importadas continuam sendo malhas, sem conversão automática para sólidos.

## Ensaios sem janela

```sh
cargo run -p nator-prototype --no-default-features --locked -- --validate-project target/project-check/run-001
cargo run -p nator-prototype --no-default-features --locked -- --inspect-project target/project-check/run-001/support.3dnator
```

O diretório deve ser novo. O ensaio cria uma peça com dois corpos, aplica posições locais/de montagem, salva o projeto, exercita desfazer/refazer, grava recuperação, remove o STL original, reabre e exporta um corpo. Gera `report.json`, `support.3dnator`, `recovery.3dnator` e `body.stl`.

A CLI anterior de STL continua disponível:

```sh
cargo run -p nator-prototype --no-default-features --locked -- --export-reference reference.stl
cargo run -p nator-prototype --no-default-features --locked -- --inspect reference.stl
```

Capturas reais usando o mesmo renderizador/composição da aplicação, sem servidor gráfico:

```sh
cargo run -p nator-prototype --locked -- --render-preview target/project-check/solid.png
cargo run -p nator-prototype --locked -- --render-perspective-preview target/project-check/perspective.png
cargo run -p nator-prototype --locked -- --render-project-preview target/project-check/run-001/support.3dnator target/project-check/editor.png
```

Os PNGs devem ter destinos novos. As capturas do renderizador verificam superfície, triangulação e transparência na projeção escolhida. A captura da interface carrega o projeto, seleciona seu primeiro corpo e acrescenta uma aba auxiliar vazia apenas para a visualização; se houver rascunho, abre seu editor. Não modifica o projeto de origem nem grava recuperação. Essas capturas não substituem o teste interativo de teclado, mouse ou diálogos do sistema.

## Medições de leveza

```sh
cargo run --release -p nator-prototype --features occt -- --benchmark target/bench/run-001
NATOR_BENCH_GPU=high cargo run --release -p nator-prototype --features occt -- --benchmark target/bench/run-002
```

Mede abertura de projeto, preparação da cena, envio à GPU, tempo de quadro em órbita contínua e memória (RSS atual e pico, no Linux) nos cenários de peça simples, malha STL importada de 1 milhão de triângulos e 400 instâncias, além do recálculo paramétrico quando o OCCT está habilitado. Cada cenário roda em um processo filho, para que o pico de memória seja por cenário. A GPU integrada é preferida por padrão; `NATOR_BENCH_GPU=high` escolhe a dedicada. O relatório inclui CPU, memória total, adaptador, driver e perfil de compilação. Resultados em [VALIDATION.md](VALIDATION.md).

## OCCT e STEP

O SDK OCCT continua opcional e externo. A ponte usa CMake e bibliotecas compartilhadas OCCT 7.x (alvo 7.6 a 7.9), com API Rust própria. Configure `OpenCASCADE_DIR` para a pasta de `OpenCASCADEConfig.cmake`, ou `CMAKE_PREFIX_PATH` para o prefixo de instalação, além do caminho de bibliotecas do sistema. No Windows, a pasta dos DLLs precisa estar no PATH.

A ponte foi compilada e validada no Linux com OCCT 7.9.3 (pacote `opencascade` do Arch). Em Arch Linux, `sudo pacman -S opencascade` instala um SDK compatível. Sem acesso de administrador, o pacote oficial pode ser extraído em uma pasta local e indicado por `OpenCASCADE_DIR` (`<prefixo>/usr/lib/cmake/opencascade`) e `LD_LIBRARY_PATH` (`<prefixo>/usr/lib`), desde que as dependências do pacote já estejam instaladas no sistema.

A receita prevista para Ubuntu 24.04 usa `cmake`, `libocct-data-exchange-dev` e `libocct-modeling-algorithms-dev` (OCCT 7.6). O workflow Windows usa conda-forge com `occt>=7.8,<8`, CMake e MSVC. Essas duas receitas ainda não foram executadas.

```sh
cargo run -p nator-prototype --no-default-features --features occt --locked -- --validate validation-output/occt-run-001
cargo run -p nator-prototype --no-default-features --features occt --locked -- --validate-part validation-output/part-001
cargo test -p nator-kernel -p nator-commands --features occt --locked
```

`--validate` verifica volumes, cortes, chanfro/arredondamento e ida e volta STEP/STL com primitivas. `--validate-part` executa o critério de entrega da etapa 2 com comandos do documento: suporte em L cotado por parâmetros, extrusão, furo, arredondamento, alteração das cotas com recálculo, desfazer/refazer, salvar, reabrir e exportar STEP e STL, conferindo volume analítico, dimensões e topologia da malha. Os testes com `--features occt` cobrem a nomeação estável de faces e as referências do histórico.

A importação STEP usa XCAF para preservar nomes, hierarquia, definições compartilhadas, posições e cores sólidas por corpo ou ocorrência. Na construção, as raízes entram como subpeças independentes da peça ativa. O projeto guarda os B-reps e as malhas, sem vínculo com o arquivo STEP. Faces importadas recebem nomes posicionais estáveis, porque a importação não é recalculada.

O diálogo de exportação oferece STEP por corpo, peça ou montagem visível. A exportação usa os B-reps e mantém cores e compartilhamento. Corpos próprios de uma peça que também tem subpeças podem aparecer como componentes sólidos no STEP. Raízes posicionadas podem receber um contêiner para separar a transformação da definição. Malhas visíveis no escopo geram erro explícito: use STL ou selecione apenas sólidos. Cores por face, materiais, superfícies abertas e STEP com arquivos externos não são suportados.

```sh
cargo run -p nator-prototype --no-default-features --features occt --locked -- --validate-step target/step-check/run-001
```

Sem o SDK e a feature `occt`, CAD/STEP falham explicitamente; não há núcleo simulado.

## Verificações e plataformas

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Resultados locais e limitações estão em [VALIDATION.md](VALIDATION.md). A CI inclui Rust e o ciclo de projeto em Linux/Windows, além dos jobs OCCT; os workflows estão preparados e ainda não foram executados remotamente. Não foi possível testar os diálogos nativos nem a janela interativa por falta de acesso ao compositor/servidor X. A captura real de interface usa renderização por software.

## Dependências

O código próprio usa MIT; as dependências mantêm suas licenças, listadas inicialmente em [THIRD_PARTY.md](../THIRD_PARTY.md). O SDK OCCT não está incorporado ao repositório. Os avisos/textos das dependências ainda precisam integrar o empacotamento de binários.

O lock mantém `clipboard-win` 5.4.1, marcado como yanked no índice do cache offline. Essa cadeia deve ser atualizada e validada antes de distribuir Windows. O terminal continua sem acesso de rede para obter novas versões ou instalar OCCT.
