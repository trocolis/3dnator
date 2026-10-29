# Execução autônoma

## STEP com pintura por região e exportação pela árvore — 29/10/2026

- Exportação STEP conserva cores sRGB por face e células de rascunho, usando as mesmas máscaras da exibição. Subdivisões são costuradas numa cópia de cada sólido, com conferência de fechamento, validade e volume, incluindo furos e cavidades. Origem/material acompanha as faces modificadas; cor explícita preserva precedência e faces sem pintura mantêm herança de corpo/ocorrência.
- Menus de contexto dos corpos e peças oferecem STEP e STL. Nome sugerido STEP vem do elemento clicado; painel STL abre no escopo correto e mantém qualidade/resumo. Corpo de malha tem STEP desabilitado.
- Aprovados 341 testes com OCCT, 257 sem OCCT, 40 ensaios gráficos, formatação e Clippy nas duas configurações. Cinco regressões de pintura STEP com releitura XCAF; logs em `target/codex-validation/step-colors-20261029/`. Importação de cores por face no documento, diálogo nativo e Windows não exercitados nesta entrega. Aplicativo 0.3.0 em desenvolvimento e formato 29 preservados; sem release ou publicação remota.

## Destaque das áreas delimitadas por rascunhos — 29/10/2026

- Seleção de faces em Peça e ferramenta Pintar destacam somente a célula sob o cursor, com o mesmo recorte da pintura: furos, interseções e exterior dos contornos. Perfis livres continuam disponíveis na peça; Pintar exige apoio. Cálculo em trabalhador único, cache por cena/raio e reaproveitamento da célula, respeitando rascunhos ocultos e oclusão por outros corpos.
- Aprovados 336 testes com OCCT, 257 sem OCCT, 40 ensaios gráficos, formatação e Clippy nas duas configurações. Capturas `part-cell.png` e `paint-cell.png` inspecionadas em `target/codex-validation/cell-hover-20261029/`.
- Sem modificação do documento ou formato 29; aplicativo 0.3.0 em desenvolvimento. Sem release, publicação remota ou validação Windows.

## Refinamento de Pintar e correções de referência/cor — 29/10/2026

- Balde de tinta com seleção múltipla por cliques, corpo inteiro por duplo clique e delimitação automática das células dos rascunhos visíveis sobre a face. Painel reduzido a superfícies e paleta; duas linhas de predefinições e personalizada hexadecimal persistente, independente da cor ativa.
- Referências de pintura deixam de depender de nomes ambíguos: nomes únicos ou índices com assinatura representam a superfície exata. Cor de base para corpo inteiro e transferência de pinturas às faces herdadas/novas da extrusão corrigem o reinício de cores após modelar. Formato 29 mantém leitura dos anteriores.
- Aprovados 335 testes com OCCT, 257 sem OCCT, 39 ensaios gráficos, formatação e Clippy nas duas configurações. Captura `paint-bucket.png` conferida em `target/codex-validation/paint-bucket-20261029/`. Preferência real não alterada pelos testes. Sem release, publicação remota ou validação Windows.

## Refinamento da extrusão e preservação de furos — 29/10/2026

- Distância inicial zero com guia independente do prisma, seta visível/arrastável e confirmação bloqueada em zero. Seleção de várias áreas permanece ativa; clicar em outro plano define o limite sem substituir os perfis iniciais. Limites por face, plano de rascunho e plano de trabalho são recalculados e persistidos no formato 28.
- Extrusões de rascunho no modo Unir preservam os contornos internos da face de apoio. Prismas exatos dos furos recortam a saliência sem limitar sua expansão externa. Edição do diâmetro, volume, seleção através do vazio, histórico e persistência aprovados.
- Aprovados 330 testes com OCCT, 255 sem OCCT, 39 ensaios gráficos, formatação e Clippy nas duas configurações. Captura `zero-arrow.png` conferida em `target/codex-validation/extrusion-20261029/`. Aplicativo 0.3.0 em desenvolvimento; sem release, publicação remota ou validação Windows.

## Pintura de faces, Projetar e gestos diretos — 29/10/2026

- Concluídos os quatro novos itens do AJUSTE.md: arraste/redimensionamento direto de imagens também no modelo; vértices imaginários de centro durante o desenho; Projetar com seleção transitória, lista, alternância pela origem/projeção e Finalizar/Cancelar; Pintar com paleta, RGB hexadecimal e recorte pelas áreas de rascunhos.
- Aparência precisa por face em camadas, separada da geometria de modelagem. Prismas de áreas com furos delimitam as cores; recálculo passa pelo trabalhador de comandos e persiste no formato 27. Seleção das projeções usa coordenadas de montagem e profundidade das faces.
- Aprovados 328 testes com OCCT, 255 sem OCCT, 39 ensaios gráficos seriais, formatação e Clippy com/sem OCCT. Captura `paint-panel.png` conferida e leitura de projeto pintado sem OCCT aprovada. Evidências em `target/codex-validation/paint-projection-20261029/`. Aplicativo 0.3.0 em desenvolvimento; sem release, publicação remota ou validação Windows.

## Cubo azul compacto e rotação pela face predominante — 27/10/2026

- Reduzido o controle para 80%, restaurando azuis claros, nomes sobre as faces e Home de contorno. Enquadrar abaixo de Home reutiliza a ação contextual do editor para modelo ou rascunho, sem abrir novamente a barra da câmera.
- Giros das setas e do menu em 45° pela normal da face principal predominante. A câmera conserva o eixo durante a sequência e reinicia a referência após outra orientação; alvo, escala e projeção ficam intactos. Arraste livre preservado.
- Aprovados 181 testes sem OCCT, 244 com OCCT, doze ensaios gráficos (61 testes da aplicação no total), formatação, Clippy nas duas configurações e compilação desktop com OCCT. Capturas conferidas em `target/codex-validation/view-cube-blue-80-20261027/`. Sem alteração de formato, release ou publicação remota.

## Cubo de vista segundo a imagem e remoção da barra do canvas — 27/10/2026

- Retirada a barra permanente da câmera. Navegador redesenhado em cinza, com chanfros largos, Home preenchido e duas setas preenchidas; nomes das faces em dicas. Shift/botão secundário inverte a órbita, e o menu do cubo reúne projeção, enquadramento e giros de 90°.
- Conservadas as 26 orientações, seleção por polígono, arraste e restauração Home. A paleta de rascunho inicia abaixo do novo controle e recolhida em telas curtas. Capturas em `target/codex-validation/view-cube-reference-20261027/` conferidas nas duas dimensões e no modo rascunho.
- Aprovados 178 testes sem OCCT, 241 com OCCT, doze ensaios gráficos (60 testes da aplicação no total), formatação, Clippy nas duas configurações e compilação desktop com OCCT. Sem alteração de formato, release ou publicação remota; imagem do usuário preservada fora do commit.

## Captura dos apoios projetados no plano do rascunho — 27/10/2026

- Substituída a projeção inicial de vértices amostrados pela consulta dos apoios geométricos completos, incluindo pontos médios de arestas e centros triangulares/retangulares. Composição de posições antes da projeção ortogonal; sem limite às primeiras oito ocorrências ou 64 vértices.
- Consulta no trabalhador, com cache por cena, plano e posição editada, respeitando fontes visíveis e ocorrências permitidas. Outros rascunhos fornecem apoios; o rascunho ativo mantém sua captura local. Resultados antigos são descartados após mudanças.
- Regressões geométricas, cache e novo ensaio gráfico validam a captura de sólidos em outro plano e a atualização após mover a origem. Aprovados 177 testes sem OCCT, 240 com OCCT, doze ensaios gráficos, formatação, Clippy nas duas configurações e compilação desktop no Linux. Sem migração, release ou publicação remota.

## Vértices de encontro, aparador e centros de polígonos — 27/10/2026

- Encontros finitos de linhas, círculos e arcos viram vértices visíveis e capturáveis. Aparar realça/remove o trecho entre encontros/extremos, trata periodicidade e sentido dos arcos e materializa os extremos com aplicação atômica e histórico de comandos.
- Triângulos oferecem baricentro; retângulos, encontro das diagonais. Apoios no desenho, construção/referência e movimento ponto a ponto, com consulta exata de faces CAD e reconstrução coplanar de malhas, preservando centros circulares e pontos médios.
- Cache de referências fora da thread da interface, invalidado também nas prévias sem mudança de revisão. Regressões geométricas e novo fluxo gráfico cobrem captura, corte, cancelamento da prévia, desfazer/refazer e persistência.
- Aprovados 173 testes sem OCCT, 235 com OCCT, onze ensaios gráficos, formatação, Clippy nas duas configurações e compilação desktop no Linux. Sem alteração do formato, release ou publicação remota.

## Rascunhos nas faces triangulares do chanfro — 27/10/2026

- Reproduzida a falha nas oito quinas sem nome do cubo chanfrado. Como o suporte do rascunho é um plano fixo, a consulta passa a usar a face exata da geometria selecionada, sem exigir nome estável e sem persistir índices topológicos.
- Compatível com projetos existentes e com posições compostas do corpo/ocorrência. Regressão percorre as 26 faces, cria os oito rascunhos, desfaz/refaz, recalcula o chanfro e verifica persistência dos planos. Seleções para referências associativas conservam a validação por nome; faces curvas continuam recusadas.
- Aprovados 162 testes sem OCCT, 220 com OCCT, dez ensaios gráficos, formatação, Clippy nas duas configurações e compilação desktop com OCCT no Linux. Sem release, migração ou publicação remota.

## Política de mensagens e erros — 27/10/2026

- Informações e avisos agora permanecem na barra de status. Apenas erros abrem diálogos e entram na fila de confirmação; o diagnóstico paramétrico conserva seu diálogo próprio.
- Espaço dinâmico e truncamento com dica preservam o rodapé em uma linha, mesmo para mensagens longas. Testes de estado e ensaio gráfico atualizados para o novo comportamento.
- Aprovados 37 testes da aplicação sem OCCT e 54 com OCCT, incluindo os dez gráficos em série, além de formatação, Clippy nas duas configurações e compilação desktop no Linux.
- Sem release, migração ou publicação remota.

## Árvore, rodapé único e Home — 27/10/2026

- Árvore sem título e com linhas menores; raiz com ícone do aplicativo e visibilidade obrigatória, normalizada em arquivos legados. Rodapé reúne Histórico, desfazer/refazer, linha do tempo e indicador curto, sem unidade. Recuperação e precisão ficam em Arquivo.
- Mensagens e alertas em diálogos com fila, separados de Pronto/Processando. Confirmação de aviso precede o teclado dos painéis; não aplica operações por trás da janela nem descarta a mensagem no quadro em que surgiu.
- Controlador de vista com Home (isométrica e enquadramento), setas curvas de órbita em 45° e giro da tela em 90°. Capturas da árvore, histórico e diálogo inspecionadas em `target/codex-validation/compact-shell-20261027/`.
- Aprovados 162 testes sem OCCT, 219 com OCCT, os dez ensaios gráficos, formatação, Clippy nas duas configurações e compilação desktop com OCCT. Capturas finais 1440 × 900 e 900 × 600.
- Sem release, migração ou publicação remota. O esboço fornecido permanece fora do commit.

## Importação do STEP ESP32 Wroom — 27/10/2026

- Reproduzido o erro no modelo real: os 142 sólidos são válidos, mas a tesselação contém triângulos colapsados com vértices repetidos. A adaptação Rust da malha agora descarta apenas elementos de área exatamente zero, preservando o B-rep, os detalhes com área positiva e os índices de face dos demais triângulos.
- Três regressões puras e ensaio optativo de STEP externo verificam conversão, validação, preservação de todos os sólidos/volumes, persistência e desfazer/refazer. O modelo passou com 142 corpos e 69.904 triângulos; projeto salvo/reaberto e captura real do editor inspecionada em `target/codex-validation/step-esp32-20261027/`.
- Aprovados 160 testes sem OCCT, 217 com OCCT e o ensaio externo, além de formatação, Clippy nas duas configurações e compilação desktop com OCCT.
- Sem alteração do arquivo externo, release, migração ou publicação remota. Validação local em Linux.

## Apoios nas arestas, círculos e arcos — 27/10/2026

- Acrescentados pontos médios das arestas, quatro quadrantes das bordas circulares e descoberta dos centros virtuais dos arcos no plano da curva. A metade do arco permanece sobre a curva, distinta do centro virtual. Malhas oferecem pontos médios nas arestas vivas e excluem diagonais internas da triangulação.
- Rascunhos de referência oferecem também pontos médios das linhas/arcos e quadrantes dos círculos. Novos rótulos de captura centralizados na tradução; consultas nativas seguem em segundo plano e o clique compartilha a escolha com o realce.
- Aprovados 157 testes sem OCCT, 214 com OCCT e os nove ensaios gráficos em série. Regressões verificam captura dos apoios na origem e no destino, confirmação e desfazer/refazer. Formatação, Clippy nas duas configurações e compilação desktop com OCCT aprovados. Sem release, migração ou publicação remota.

## Centros circulares e captura no movimento ponto a ponto — 27/10/2026

- Acrescentados centros virtuais exatos das bordas circulares, no plano de cada superfície, inclusive no vazio de furos. A região revela o centro e a proximidade captura a mira em alcance constante na tela; realce e clique compartilham a escolha na origem e no destino.
- Consultas do B-rep fora da interface, com cache por aba/revisão e descarte de entregas antigas. Costuras dos cilindros não competem como vértices artificiais. Captura considera poses, visibilidade, bloqueio, isolamento, seção e oclusão; referências dos rascunhos permanecem prioritárias no destino.
- Aprovados 155 testes sem OCCT, 208 com OCCT e os oito ensaios gráficos em série, incluindo alinhamento arruela/disco por eventos reais egui e desfazer/refazer. Formatação, Clippy nas duas configurações e compilação desktop com OCCT aprovados. Sem release, alteração de formato ou publicação remota; Windows e janela nativa não exercitados.

## Extrusão interativa e modos automáticos — 27/10/2026

- Perfil pode ser uma célula fechada do rascunho ou uma face plana do sólido, sem criar rascunhos auxiliares. Arraste da seta define a distância; clicar em face após o perfil define Até face automaticamente, com recorte exato em planos inclinados. Mantidos um/dois lados, simetria, deslocamento inicial, expressões e substituição manual da operação.
- Classificação no núcleo de comandos: Novo corpo no espaço livre, Unir ao sair de uma superfície e Remover ao entrar no material. A prévia de remoção soma o corpo restante ao volume efetivamente retirado, em vermelho translúcido, e não é publicada até OK. Cache assíncrono de áreas por aba/revisão evita bloquear a interface ou reutilizar geometria obsoleta.
- Formato 17 registra células orientadas e extrusão de faces; leitura anterior preservada. Regressões incluem furos, círculos sobrepostos, recálculo, limite inclinado, volume subtraído exato, modo manual, salvamento/clonagem e referências a laterais derivadas. Verificações finais: 153 testes sem OCCT, 200 com OCCT e sete ensaios gráficos em série, com formatação, Clippy nas duas configurações e compilação desktop com OCCT aprovados. Evidências em `target/codex-validation/smart-extrusion-20261026/` (trabalho iniciado em 26/10). Aplicativo 0.3.0; sem release/publicação remota, Windows ou janela nativa exercitados.

## Cotas durante o desenho — 26/10/2026

- Entradas junto à prévia para linha (comprimento/ângulo horizontal), retângulo (largura/altura), círculo (raio) e arco (raio/orientação inicial, depois raio/abertura). Digitar trava a medida, apagar libera, Tab/Shift+Tab alterna, Enter confirma e Esc descarta. Valores inválidos bloqueiam a criação; capturas incompatíveis com a ponta dimensionada são descartadas.
- Cálculo de prévia em `sketch::drafting`, independente de egui. Cotas digitadas são publicadas por `UpdateSketch` após resolução no trabalhador. Acrescentada restrição angular, com derivada circular na passagem por ±π, resolução de expressões de ângulo, edição/referência e persistência no formato 16, mantendo leitura de 1 a 15.
- Aprovados 150 testes sem OCCT, 192 com OCCT e seis ensaios gráficos em série. Regressão gráfica verifica medidas, digitação direta, Tab/Shift+Tab, liberação do campo, atualização pelo cursor, Enter único, erros, Esc e desfazer/refazer. Formatação, Clippy nas duas configurações e compilação desktop com OCCT aprovados. Evidências em `target/codex-validation/dynamic-sketch-20261026/`; versão do aplicativo 0.3.0 preservada, sem release/publicação remota. Windows e janela nativa não exercitados.

## Faixa contextual Rascunho — 26/10/2026

- Consultados o layout e o estudo visual de rascunho do SolidCraft local. Aplicados Criar, Modificar, Padrão e Restringir, com Cota/grade 3 × 2, menus nos rótulos e Concluir fixo à direita; Reference e Configure omitidos. Código e ícones próprios, sem copiar recursos externos.
- Ferramentas existentes passaram aos painéis de comando com confirmação e seleção no viewport. Intenções e validação atômica ficam em `nator_sketch::edit`, sem egui; resolução e publicação continuam pelo trabalhador e por `UpdateSketch`. A janela contextual fica restrita a estado, reparo e lista de restrições. Limitações de seleção/padrões/tangência documentadas em `INTERFACE.md`.
- Aprovados 146 testes sem OCCT, 188 com OCCT e cinco ensaios gráficos em série, incluindo aplicação/desfazer de cota e deslocamento e cancelamento de padrão. Formatação, Clippy nas duas configurações e compilação desktop com OCCT aprovados. Capturas 1440 × 900 e 900 × 600 em `target/codex-validation/sketch-ribbon-20261026/`. Sem release, migração ou publicação remota; Windows e janela nativa não exercitados.

## Área de edição sem Propriedades lateral — 26/10/2026

- Eliminado o painel lateral Propriedades, seu botão na faixa, sua ação na busca e os campos legados de nome/posição da seleção. O viewport agora ocupa todo o espaço à direita da árvore.
- Preservados os controles de rascunho em janela contextual, a medição em janela própria e a edição de cor no menu de contexto dos corpos. Dimensões e informações da malha ficam na janela Medir seleção; renomear, mover, exportar e excluir continuam nos comandos existentes.
- Aprovados 140 testes sem OCCT, 182 com OCCT, os quatro ensaios gráficos em série, formatação e Clippy nas duas configurações e compilação desktop com OCCT. Capturas de modelo, rascunho e medição inspecionadas em `target/codex-validation/remove-properties-20261026/`. Sem release, migração ou publicação remota.

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
