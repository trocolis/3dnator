# Execução autônoma

## Subopções de Mover na aba Peça — 26/10/2026

- Mover na Peça agora abre o mesmo menu da Montagem: Livre, Face a face, Aresta a aresta e Vértice a vértice. As opções compartilham uma definição; Livre mantém a distinção entre corpos e componentes. O grupo Modificar conserva Copiar, Combinar e Dividir ao lado do menu.
- Alinhamentos durante a construção recusam origens de outras peças e continuam aceitando geometria externa como destino. Ensaio gráfico ampliado verifica as quatro entradas, alinhamento de corpos, instâncias preservadas e desfazer, além da recusa de origem externa.
- Aprovados os 30 testes da aplicação com e sem OCCT, os quatro ensaios gráficos em série, formatação, Clippy nas duas configurações e compilação desktop com OCCT. Menus capturados e inspecionados em `target/codex-validation/part-move-menu-20261026/`; cenário novo `part-menu-move`. Sem release, migração ou publicação remota.

## Faixa sem Inserir e Vista — 26/10/2026

- Removidas as abas Inserir e Vista e seus grupos da faixa, com ajuste do cenário de captura de inserção para Montagem. A faixa mantém Montagem, Peça e Inspecionar, além de Rascunho contextual. Busca e controles existentes conservam acesso às ferramentas.
- Aprovados os 30 testes da aplicação nas configurações com e sem OCCT, formatação, Clippy do workspace nas duas configurações e compilação desktop com OCCT. Captura da faixa inspecionada e cenário de inserção executado; evidências em `target/codex-validation/remove-tabs-20261026/`.

## Reorganização da aba Peça — 26/10/2026

- Aba Modelo renomeada para Peça e ferramentas reagrupadas conforme os itens definidos: Novo rascunho; Extrusão/Revolução/Furo; Arredondar/Chanfro/Casca; Mover/Copiar/Combinar/Dividir. Mover e Copiar abrem modos próprios; a cópia não desloca o original. Ferramentas retiradas da faixa continuam na busca e parâmetros em Inspecionar.
- Ensaio novo com cliques reais na faixa verifica cópia, movimento, preservação do original e desfazer. Aprovados 140 testes sem OCCT, 182 com OCCT, quatro ensaios gráficos em série, formatação e Clippy nas duas configurações. Captura inspecionada e compilação desktop com OCCT aprovadas; logs e imagem em `target/codex-validation/part-ribbon-20261026/`.
- Pendente de esclarecimento: significado de “Plano extendido” (plano de referência deslocado, exibição ampliada dos planos da origem ou menu de construção de planos). A pergunta foi apresentada durante a implementação dos demais itens.

## Cancelamento de prévias e restauração da cena — 26/10/2026

- Corrigido o relato de cópia que continuava visível após cancelar. Dois fatores se combinavam: a prévia era encerrada depois da verificação da cena, sem pedir redesenho, e uma solicitação de cena concluída continuava bloqueando a mesma chave ao voltar ao documento. A chave pendente agora é liberada na entrega da cena calculada e a transição da prévia para o documento solicita redesenho sem criar laço ocioso.
- Regressões sem GPU verificam agendamento e ociosidade. Ensaio com egui/wgpu reproduz cópia, Cancelar e Esc, restaura malha/limites e verifica documento, salvamento e desfazer sem alterações. Aprovados 140 testes sem OCCT, 182 com OCCT, formatação e Clippy nas duas configurações; os três ensaios gráficos passaram em série. A tentativa gráfica simultânea encerrou com SIGSEGV e está registrada em [VALIDATION.md](VALIDATION.md), com seus logs.
- Evidências em `target/codex-validation/cancel-preview-20261026/`. Sem release, migração ou publicação remota.

## Corpos atuais e etapas geométricas — 26/10/2026

- Investigado o relato de que modificar um corpo acrescentava outro e ocultava o original. O documento preserva resultados por operação para recalcular; a árvore apresentava também essas etapas intermediárias ocultas. A pasta Corpos e seu contador agora listam os resultados atuais, mantendo acessíveis corpos independentes ocultados manualmente e etapas antigas explicitamente mostradas.
- Modificações herdam nome e cor da entrada principal; o nome da operação permanece na linha do tempo. Edição de operação existente verificada sem acrescentar resultados, com preservação da renomeação do corpo e de salvar/reabrir/desfazer/refazer.
- Duas regressões novas aprovadas; 138 testes sem OCCT e 180 com OCCT aprovados, além de formatação, Clippy nas duas configurações e compilação desktop com OCCT. Ensaio da peça paramétrica e captura real da árvore com um corpo/furo/arredondamento aprovados em `target/codex-validation/body-history-20261026/`. Sem migração de formato, release ou publicação remota.

## Revisão dos incrementos do Claude e campos de montagem — 26/10/2026

- Revisados os três commits recentes de fluxo de montagem, posicionamento automático e árvore com pastas. Mantidas as decisões de interface e a separação entre documento, comandos e apresentação; executado o conjunto de testes antes de refinar os campos numéricos.
- Corrigidos descarte de texto inválido ao perder foco, confirmação com último valor válido e truncamento silencioso fora da faixa. O estado transitório agora pertence ao painel; Mover e origem do padrão circular bloqueiam comandos enquanto houver erro. Esc restaura o campo sem fechar o painel; Enter só confirma depois de concluir a digitação.
- Quatro regressões novas de interação egui, além da ampliação do teste de expressões. Aprovados 137 testes sem OCCT, 178 com OCCT, formatação, Clippy nas duas configurações e compilação desktop com OCCT. Capturas reais de árvore e Mover inspecionadas em `target/codex-validation/refinement-20261026/`.
- Como próxima frente de revisão da árvore, conferir o vínculo entre a ocorrência escolhida e a edição de definições compartilhadas: o lápis recebe a definição da peça e a resolução do referencial ainda consulta seleção/último clique. Este incremento não altera essa escolha. Windows e interação nativa continuam pendentes; sem release ou publicação remota.

## Rascunhos integrados à área de edição — 26/10/2026

- Atendida a solicitação de manter o modelo na mesma tela ao criar/editar rascunhos. A câmera alinha suavemente ao suporte; projeção, cursor e desenho respeitam o plano da peça e a posição na montagem. A última cena válida é mantida enquanto a revisão seguinte é preparada.
- Novo rascunho agora solicita XY/XZ/YZ na origem ou face plana do sólido, com cancelamento sem alteração do documento. Seleção de face exata em segundo plano; origem local consistente entre cliques/ocorrências. O plano da face é capturado, sem vínculo paramétrico posterior.
- Origem visível com olho na árvore; rascunhos desenhados também fora da edição. Extrusão/revolução/furo ocultam o perfil na transação bem-sucedida; olhos na árvore reexibem os perfis sem abrir a edição. Recálculo preserva a escolha, que participa de desfazer/refazer e persistência.
- 111 testes sem OCCT e 146 com OCCT aprovados, além de um ensaio de interação egui/wgpu que acompanha seleção, desenho, conclusão, extrusão, reexibição e face mantendo o mesmo viewport e o modelo. Formatação/Clippy/compilação aprovados; capturas em dois tamanhos e ensaios CLI de peça/rascunho registrados em `target/codex-validation/sketch-viewport-20261026/`.
- Uso documentado em [INTERFACE.md](INTERFACE.md). Windows continua adiado, sem publicação remota ou release; versão de desenvolvimento 0.3.0 e formato 11 preservados.


Autorização do usuário: continuar todas as etapas sem solicitar acompanhamento.

## Navegador de orientação e giro da tela — 26/10/2026

- Usuário relatou desenho/controle imprecisos do cubo, indicador XYZ sobreposto e linhas que atravessavam a área de visualização; pediu arraste e setas de giro para planos em pé.
- Substituída a geometria sobreposta por poliedro fechado com seis faces, doze chanfros e oito cantos. Seleção pelos polígonos reais, pintura por triângulos e segmentos limitados ao recorte do controle, com descarte de degenerações. Removidas as linhas XYZ; nomes das vistas acompanham a face.
- Arraste esquerdo orbita sem selecionar ou mover corpos. Câmera com inclinação da tela, órbita contínua pelos polos e base compartilhada por projeção, seleção, pan e zoom. Setas avançam ao próximo múltiplo de 90° e clicar no ângulo retorna a 0°; vistas superior/inferior coerentes entre cubo e atalhos. Estado acompanha a câmera da aba, sem alterar o documento ou formato de projeto.
- 105 testes sem OCCT e 137 com OCCT 7.9.3 aprovados; formatação e Clippy aprovados nas duas configurações. Regressões verificam as 26 direções, eventos de clique/arraste/setas, consistência com seleção/GPU e tesselação limitada em 832 orientações. Capturas reais em dois tamanhos e ensaios ortográfico/perspectiva aprovados em `target/codex-validation/view-cube-20261026/`.
- Documentação de uso em [INTERFACE.md](INTERFACE.md). Windows permanece adiado; sem publicação remota ou release.

## Janela sem barra de título nativa — 26/10/2026

- Usuário confirmou que os controles devem ficar na barra escura superior, imediatamente abaixo da antiga barra de título. Decoração nativa removida; minimizar, maximizar/restaurar e fechar integrados à direita dessa barra.
- Implementados arraste do fundo, duplo clique para maximizar/restaurar e alças nas bordas/cantos para redimensionar. Fechar continua pelo fluxo do editor, aguardando a tarefa e preservando Salvar/Descartar/Cancelar para projetos modificados.
- Quatro regressões novas aprovadas; nove testes da aplicação aprovados com/sem OCCT. Formatação e Clippy aprovados nas duas configurações. Capturas 1280 × 820 e 900 × 600 inspecionadas em `target/codex-validation/window-frame-20261026/`; nenhuma validação Windows ou de gerenciador de janelas foi declarada sem execução.

## Refinamento da interface e prioridades — 26/10/2026

- Usuário adiou a prioridade de testes Windows e solicitou polimento da interface com referência no projeto local SolidCraft. O suporte Windows permanece preparado, com CI nativa opcional por `workflow_dispatch`/`validate_windows`.
- Implementados tema claro com barra escura, ícones vetoriais originais, faixa de tarefas (Modelo/Montagem/Inserir/Inspecionar/Vista e Rascunho contextual), busca por S, painéis recolhíveis/roláveis, árvore agrupada com visibilidade e histórico por ícones. Operações acessadas diretamente mantêm publicação por comandos e confirmação nos editores.
- Ajustados contraste de erros/restrições/medições, fundo do renderizador e esmaecimento conforme o tema. A revisão de 900 × 600 identificou rascunhos cortados; implementados enquadramento inicial e botão Enquadrar no modo 2D, considerando os limites radiais.
- 91 testes sem OCCT e 123 com OCCT 7.9.3 aprovados; formatação e Clippy aprovados nas duas configurações. Ensaios CLI de núcleo, projeto, rascunho, editor, primeira peça e STEP aprovados. Capturas reais em dois tamanhos e nas duas projeções inspecionadas, sem erros wgpu, em `target/codex-validation/interface-20261026/`.
- Guia em [INTERFACE.md](INTERFACE.md). Nenhuma release ou publicação remota; Windows e interação com diálogos nativos não foram declarados aprovados.

## Auditoria e correções — 26/10/2026

- Conferido o plano contra código, testes, ensaios e critérios de aceite. As funcionalidades principais estão implementadas; o plano **não tem aceite completo** por falta de execução Windows/Ubuntu, interação real e medições no hardware físico de 8 GB. Detalhes em [PLAN_AUDIT.md](PLAN_AUDIT.md).
- Corrigidos empacotadores para preservar destinos/arquivos existentes, rejeitar dependências ausentes e resolver os imports transitivos do SDK no Windows; Linux resolve o diretório explícito antes de `ldd`, inclui TBB quando necessário e evita symlinks externos. Textos integrais LGPL/exceção OCCT agora versionados e incluídos, com README próprio do pacote e manifesto de dependências.
- Cinco regressões Linux aprovadas. Duas regressões Windows e os ensaios de pacote movido com PATH limpo preparados na CI, que passou a compilar/testar também o desktop com OCCT. Nenhum resultado Windows/Ubuntu foi declarado sem execução.
- Repetidos formatação, Clippy e testes: 87 aprovados sem OCCT e 119 com OCCT 7.9.3. Compilação desktop debug/release aprovada. Pacote real com 26 bibliotecas pelo SONAME passou nos seis ensaios CLI após relocação, com ambiente limpo; capturas do renderizador e do editor aprovadas, editor/folha inspecionados.
- Definidos limites numéricos provisórios para o aceite de desempenho. Benchmark release repetido com llvmpipe; não substitui medição em GPU integrada física. Documentação corrigida para não prometer independência de bibliotecas do host nem inventário completo de licenças que não foi produzido.
- Sem release, tag ou publicação remota. As pendências de aceite continuam abertas, com procedimento concreto de verificação.

## Continuação autônoma — cores e intercâmbio (26/10/2026)

- Reafirmada a autorização de seguir todas as etapas sem monitoramento, preservando os limites de publicação e as validações externas ao ambiente.
- Cores sRGB por corpo integradas ao documento, aos comandos e à cena, com editor nas propriedades e herança nas operações. Formato de projeto 8 com leitura das versões 1 a 7 usando a cor neutra.
- 79 testes sem OCCT e 102 com OCCT aprovados; captura wgpu em perspectiva confirmou 31.727 pixels com a cor de teste aplicada, além de triangulação e transparência.
- Próxima frente: importação/exportação STEP com nomes, cores e hierarquia pelo XCAF do OCCT; depois documentação técnica e sombras opcionais.

### STEP com cores e montagem

- Implementado intercâmbio XCAF com definições compartilhadas, nomes UTF-8, posições e cores de corpo/ocorrência, incluindo montagem no contexto de construção. Arquivos STEP com referências externas, superfícies abertas ou transformações não rígidas geram erros explícitos.
- Exportação de corpo, peça ou montagem visível; raízes posicionadas usam contêineres quando necessários para separar definição e transformação, sem acrescentar níveis em cada reexportação. Malhas não são omitidas ou convertidas silenciosamente.
- Formato de projeto 9 armazena cores de ocorrência. O intercâmbio é serializado entre threads para evitar interferência do estado global do OCCT, observada nos testes concorrentes.
- 79 testes sem OCCT e 105 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. `--validate-step` aprovado em `target/codex-validation/step-002/`; projeto `imported.3dnator`, STEP `assembly.step`, relatório `report.json` e captura real `target/codex-validation/step-editor-001.png`.
- A CI recebeu o ensaio STEP. Windows/OCCT 7.6 continuam sem execução local; a próxima frente funcional é documentação técnica (HLR, cotagem associativa e múltiplos cortes), seguida de sombras e preparação de distribuição.

### Vistas HLR do B-rep

- A camada de comandos prepara sólidos posicionados na origem da folha e pede ao OCCT a classificação conjunta de linhas visíveis/ocultas. O desenho recebe segmentos próprios em mm; não depende do núcleo. Curvas são discretizadas com controle de desvio de 0,01 mm, por intervalos C2.
- O diálogo usa a projeção precisa para contextos exclusivamente sólidos e conserva a alternativa aproximada para malhas/contextos mistos, identificando a origem na legenda. Cortes da malha continuam identificados como aproximados.
- 79 testes sem OCCT e 108 com OCCT aprovados, com formatação e Clippy aprovados. O ensaio STEP passou a gerar a folha precisa em PNG/PDF, disponível em `target/codex-validation/step-hlr-001/` e inspecionada visualmente.
- Restam na documentação técnica: cotas associativas reposicionáveis, múltiplos cortes e corte preciso do B-rep. Depois seguem sombras opcionais e preparação de distribuição. Não houve mudança no formato de projeto neste incremento.

### Múltiplos cortes e configurações da folha

- Implementados A–A e até sete cortes adicionais por folha, com identificadores estáveis e planos editáveis por origem/normal. A distribuição das células cresce com o número de cortes sem modificar a escala escolhida.
- Orientação retrato/paisagem e resolução PNG de 72 a 300 DPI persistidas por folha. Formato 10 com valores padrão para projetos anteriores.
- 82 testes sem OCCT e 111 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. Ensaio `target/codex-validation/multiple-sections-001/` gerou a folha em retrato com cortes perpendiculares A–A/B–B, inspecionada visualmente.
- Próximas tarefas funcionais: corte preciso do B-rep, cotas associativas reposicionáveis e sombras opcionais; depois preparação de distribuição. As validações externas permanecem registradas no roadmap.

### Cortes precisos do B-rep

- Interseções calculadas sobre cada corpo CAD posicionado, sem alterar sua geometria, com curvas amostradas pelo mesmo controle de desvio usado em HLR. Os dados de corte são preparados uma vez e reutilizados nas vistas e nos traços dos planos.
- Preservados furos internos e grupos por corpo na hachura; sobreposições de peças distintas não viram vazios artificiais. A legenda distingue a precisão das vistas e dos cortes.
- 83 testes sem OCCT e 113 com OCCT aprovados; formatação e Clippy aprovados. `target/codex-validation/precise-sections-001/` contém relatório, projeto, STEP e folha PNG/PDF com cortes A–A/B–B da montagem. Imagem inspecionada.

### Cotas associativas reposicionáveis

- Amarração a referências estáveis de faces e arestas do B-rep e caminhos de ocorrência em montagens. Apoios geométricos exatos (centroides, centros e raios de cilindros e esferas, extremidades e centros de arcos) calculados pelo núcleo OCCT.
- Medição de comprimentos, raios, diâmetros, distâncias entre faces paralelas e ângulos entre faces planas (com arcos em projeção ortogonal e chamadas em vistas inclinadas).
- Tolerância a alterações do modelo: referências removidas mantêm a cota com aviso explícito de revisão pendente, sem abortar a regeneração. Reposicionamento interativo por arraste de alças na folha sem reexecutar o núcleo CAD nem travar a interface. Formato `.3dnator` versão 11 com migração compatível.
- 86 testes sem OCCT e 117 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. Ensaios `--validate-project`, `--validate-sketch`, `--validate-editor`, `--validate-part` e `--validate-step` aprovados.

### Sombras opcionais no renderizador

- Sombras opcionais e desligadas por padrão, ativáveis nas opções de visualização da área 3D (`Sombras`).
- Iluminação direcional com faces opostas sombreadas e luz suave de preenchimento, combinada com sombra de contato suave no plano de apoio da cena gerada com decaimento quadrático, sem alocação de mapas de sombra pesados e mantendo leveza para GPUs integradas.
- 87 testes sem OCCT e 118 com OCCT aprovados; formatação e Clippy sem avisos. Ensaios `--render-preview` e `--render-perspective-preview` verificados pelo wgpu/Vulkan confirmando sombreamento e decaimento de brilho.

### Empacotamento, distribuição e licenças

- Arquitetura de empacotamento portátil documentada em `docs/DISTRIBUTION.md`, abordando resolução dinâmica de bibliotecas compartilhadas, compatibilidade entre distribuições Linux e runtime do Windows.
- Scripts automatizados de empacotamento em `scripts/package_linux.sh` (com launcher portátil que resolve `LD_LIBRARY_PATH`, coleta recursiva de symlinks e geração de `.tar.gz`) e `scripts/package_windows.ps1` (com DLLs adjacentes e geração de `.zip`).
- Teste real de empacotamento no Linux: pacote gerado e executado em ambiente limpo (`env -i`), confirmando o carregamento autônomo das 78 bibliotecas dinâmicas do OCCT e a execução de `--validate` com relatório aprovado.
- Inventário de licenças em `THIRD_PARTY.md` expandido com o aviso proeminente e texto integral da *Open CASCADE Technology Exception (version 1.0)* à GNU LGPL 2.1, detalhamento da garantia de vinculação dinâmica para substituição pelo usuário final e catálogo de todas as crates diretas e transitivas.
- Registro original declarava concluídas as cinco etapas no ambiente local. A auditoria posterior, acima, corrigiu falhas nos pacotes e distinguiu implementação local de aceite completo, incluindo Ubuntu, interação em janela e requisitos do host.

## Critério de trabalho

Concluir incrementos verificáveis, preservar decisões já aprovadas, executar os testes pertinentes e atualizar este registro. Não publicar o repositório, não substituir malhas por sólidos simulados e não declarar validações que não foram executadas.

## Ponto inicial

- 0.2.0: documento, comandos, persistência, recuperação, abas e corpos de malha; 22 testes aprovados.
- SDK OCCT ausente, DNS indisponível no terminal, `.git` somente leitura e janela interativa inacessível. Há captura real da interface sem janela.

## Sessão Codex (até 26/10/2026, 01h30)

Rascunhos 2D com solver, parâmetros e expressões, histórico de construção sobre a ponte OCCT (ainda sem compilar), montagem com subpeças, clones e grupos, folhas técnicas PNG/PDF. Ao fim da sessão: 39 testes aprovados, sem SDK OCCT.

## Sessão Claude Code (26/10/2026)

Executada em paralelo ao Codex até o encerramento dele. A partir daí, cada incremento foi registrado em commit próprio no repositório Git, criado nesta sessão.

1. OCCT 7.9.3 obtido sem privilégios (pacote oficial do Arch extraído em `target/`); corrigida a ordem dos componentes no CMake; primeira execução real da ponte, com `--validate` aprovado.
2. Referências topológicas estáveis: nomes de faces pela origem, regiões por entidades e arestas por pares de faces; formato `.3dnator` versão 7 com migração.
3. Ensaio `--validate-part` do critério de entrega da etapa 2 com o núcleo real.
4. Medições de leveza (`--benchmark`) na GPU integrada. Menos memória na visualização: linhas no shader, fusão exata de vértices e arestas vivas calculadas por geometria.
5. Seleção gráfica de arestas e faces pelos nomes de origem.
6. Etapa 3: furos (rebaixo, escareado, passantes, rosca registrada), padrões linear e circular, espelhamento, divisão, revolução por linha do rascunho e padrões no rascunho.
7. Etapa 4: padrões e espelhamento de instâncias e rascunho em face selecionada.
8. Etapa 5: corte A–A hachurado, revisão, data e notas de rosca nas folhas, ferramenta Medir com medidas exatas no B-rep.
9. Exportação STL com escopo, qualidade e resumo antes de gravar.
10. Arredondamento e chanfro de várias arestas, chanfro por ângulo, casca com várias faces e para fora; extrusão até uma face e com resultado combinado.
11. Controles visuais: isolamento, transparência, bloqueio de seleção e esmaecimento na construção.

Ao fim: 90 testes aprovados com OCCT e 68 sem, clippy sem avisos nas duas configurações, ensaios de linha de comando aprovados.

## Pendências

Ver [PLAN_AUDIT.md](PLAN_AUDIT.md) e a seção Situação atual de [ROADMAP.md](../ROADMAP.md): Windows e Ubuntu (CI não executada), máquina física de referência e interação real com a janela. Sombras, STEP com cores/montagem e folhas HLR estão implementados e verificados localmente; não são mais pendências funcionais.

## Continuação Codex (26/10/2026)

- Revisados código, histórico e roadmap; estado inicial limpo, com os 68 testes sem OCCT aprovados.
- Implementada a projeção em perspectiva por aba, mantendo a ortográfica como padrão, com raios de seleção, medições, movimento livre e navegação coerentes com a matriz da GPU. A vista isométrica conserva escala, alvo e projeção; a orientação escolhida no cubo é guardada na câmera da aba.
- Acrescentados cinco testes de câmera e ampliados os dois existentes para ambas as projeções: recálculo dos limites da cena, zoom no cursor, seleção, redução aparente com a distância, planos de recorte, enquadramento e movimento no plano do alvo.
- Capturas reais ortográfica, perspectiva e da interface em `target/codex-validation/`, pelo wgpu/llvmpipe, sem erros de validação. Triangulação e transparência verificadas nas duas projeções.
- Formatação e Clippy sem avisos aprovados; 73 testes sem OCCT e 95 com OCCT aprovados. Guia do protótipo atualizado para remover descrições obsoletas da versão 0.2.0.

## Continuação Codex — conjuntos espelhados (26/10/2026)

- Removida a limitação de espelhar peças com subpeças: hierarquias aninhadas com definições repetidas geram uma cópia independente, mantendo o compartilhamento interno e compensando a reflexão nas posições rígidas de cada ocorrência.
- Preservados grupos internos durante a clonagem e o espelhamento, e a visibilidade de ocorrências e corpos finais ocultos. Grupos do nível externo não são copiados para dentro do conjunto.
- Corrigida a extração dos ângulos de matrizes perto de ±90°, usando `atan2` com a projeção do eixo em vez de `asin`; normais de planos são convertidas sem subtrair translações grandes.
- Testes cobrem reflexão em plano oblíquo no referencial de um pai rotacionado, compartilhamento por vários caminhos da hierarquia, grupos internos, visibilidade, salvamento/reabertura, duplo espelhamento, desfazer/refazer e falhas atômicas.
- Ensaio com OCCT verifica conjuntos mistos de malhas e sólidos paramétricos, propriedades exatas e alteração independente de parâmetros das subpeças após reabrir o projeto.
- Formatação e Clippy aprovados nas duas configurações; 78 testes sem OCCT e 101 com OCCT aprovados. O formato de projeto continua na versão interna 7, sem migração nova.
