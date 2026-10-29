# Interface desktop

A organização visual usa como referência o shell compacto do SolidCraft e seus estudos em `docs/design/ux-redesign/`: barra escura de documentos, faixa clara de ferramentas agrupadas por tarefa, árvore lateral e histórico na base. Código, desenhos vetoriais e textos desta entrega foram produzidos no 3dnator, sob a licença MIT do projeto.

## Organização e uso

- **Barra superior:** nome 3Dnator, menu Arquivo seguido de Novo e Salvar e projetos em abas. Os ícones da barra escura usam traço claro, destaque azul-claro e realce escuro ao passar o mouse. À direita ficam minimizar, maximizar/restaurar e fechar o aplicativo, substituindo a barra de título nativa. O ponto no nome da aba indica alterações não salvas; o fechamento continua oferecendo salvar, descartar ou cancelar. Recuperação e casas decimais ficam no menu Arquivo.
A faixa contém Montagem, Peça e Inspecionar, além de Rascunho durante a edição. Todas usam a mesma altura compacta de grupos: botões de 72 × 54 pontos, ícones de 24 pontos, legendas de até duas linhas e rodapés de 18 pontos tanto para rótulos quanto para menus. O espaço vertical entre corpo e rodapé é de 2 pontos; as margens verticais da faixa também são menores. A galeria de restrições cabe nessa mesma altura. Trocar de aba mantém a posição superior do canvas, inclusive com rolagem horizontal em janelas estreitas. Os títulos têm mais espaço lateral e área de clique de pelo menos 104 × 30 pontos; `|` separa somente abas consecutivas não selecionadas. As abas de projetos também têm mais margem lateral e o mesmo critério de separação. A faixa mostra apenas os grupos de ferramentas da aba escolhida, sem bloco fixo de identificação do espaço de trabalho. O botão de busca fica à direita da linha das abas; importações, ferramentas de visualização e mostrar/ocultar a árvore do projeto ficam na busca (também por S). Câmera e projeção ficam no menu do cubo.

**Selecionar** é o primeiro botão de **Montagem**, **Peça** e **Rascunho**, no grupo Seleção. Usa o cursor de mouse como ícone e fica destacado no modo neutro, sem desenhar um ícone adicional junto ao ponteiro no canvas. Esc cancela a ferramenta/prévia e volta a Selecionar; não encerra a edição do rascunho nem apaga elementos já confirmados. Nos campos numéricos, o primeiro Esc ainda restaura o valor em edição; depois de deixar o campo, Esc cancela a ferramenta. Ctrl+clique acrescenta/retira itens, e Ctrl no vazio conserva a seleção.

As ferramentas usam os alvos já selecionados quando compatíveis: Mover recebe todos os componentes/corpos; combinar aproveita os dois corpos escolhidos na ordem da seleção; alinhamento usa a geometria do último clique válido como origem; casca/extrusão e os suportes de rascunho/plano aproveitam faces; arredondamento/chanfro aproveitam a aresta selecionada; Cota e restrições usam as entidades escolhidas no rascunho. Selecionar apenas um corpo pela árvore não inventa uma aresta ou face: nesses casos a ferramenta espera a escolha necessária. Trocar de ferramenta descarta a prévia anterior e conserva os alvos.

- **Montagem:** primeira aba, aberta por padrão, com o fluxo próprio na ordem de trabalho: **Rascunho de referência** (Referência); **Nova peça** ▾ com Criar nova peça, Criar a partir de STEP e Criar a partir de STL (Peças); **Mover** ▾ com Livre, Face a face, Aresta a aresta e Vértice a vértice (Posicionar); **Copiar** ▾ com Nova instância, Cópia independente e Padrão de repetição; e Agrupar/Desagrupar (Organizar). Os botões com ▾ abrem a lista de subopções, cada uma com ícone e uma linha de explicação. Espelhar componente continua no menu de contexto da árvore; folha técnica e STEP ficam em Inspecionar.
- **Peça:** substitui a aba Modelo. Grupos na ordem de trabalho: **Rascunho** (Novo rascunho), **Criar** (Extrusão, Revolução, Furo), **Acabamento** (Arredondar, Chanfro, Casca), **Modificar** (Mover, Copiar, Combinar, Dividir) e, por último, **Construção** (Plano deslocado). Mover e Copiar abrem modos separados para corpos da peça ativa, com manipulador e prévia: Mover altera a posição e Copiar mantém o original. Espelhamento e padrões continuam na busca por S; parâmetros ficam em Inspecionar. As operações abrem seu editor com a ferramenta escolhida; perfil e corpo são sugeridos pelo contexto e continuam sujeitos à confirmação.
- **Inspecionar:** medição da seleção, folha técnica, parâmetros e exportação. Medir exige um elemento capturado na área 3D e abre uma janela própria; os resultados exatos continuam sendo preparados em segundo plano. A janela também informa dimensões locais, triângulos e arestas abertas.
- **Rascunho:** aba contextual selecionada ao entrar em edição. Segue a organização visual do SolidCraft, sem Reference ou Configure: **Criar** (Linha, Retângulo, Círculo, Arco, Projetar, Imagem), **Modificar** (Aparar, Deslocamento, Arredondar, Chanfrar, Espelhar, Dimencionar) e **Sair** (Concluir, Cancelar). Ferramentas e indicadores usam contornos pretos e destaques verdes. Padrão e Restringir ficam temporariamente ocultos, inclusive na busca; restrições existentes continuam acessíveis pela lista de Selecionar. Sair segue o mesmo padrão dos outros grupos: dois botões com ícones e opções também no menu do rótulo. Todos os grupos rolam juntos horizontalmente quando falta espaço. Concluir mantém o conteúdo publicado; alterações pendentes precisam ser resolvidas. Cancelar descarta a prévia e restaura o conteúdo do rascunho capturado ao entrar na edição, preservando mudanças independentes no restante do projeto. A restauração pode ser desfeita/refeita e não remove um rascunho já existente (um rascunho recém-criado retorna ao seu conteúdo inicial vazio). Sem mudanças publicadas, não cria comando no histórico. As duas opções encerram a edição e restauram a aba e a orientação anteriores.

Os nomes dos grupos com ▾ abrem menus: Criar inclui Selecionar; Modificar acrescenta Estender e Excluir entidade. Entrar no modo rascunho deixa a área de edição sem a antiga janela automática Rascunho. Escolher Linha, Retângulo, Círculo, Arco ou Aparar abre uma janela com o nome da ferramenta, abaixo do cubo. As ferramentas de criação oferecem Traço, variações com ícones próprios e a seção Medidas, sem textos explicativos no painel. As dicas ficam ao passar o mouse. A opção Selecionar do menu Criar continua abrindo diagnóstico, graus de liberdade, reparo e lista de restrições, com exclusão e edição de cotas; o primeiro botão Selecionar da faixa ativa a seleção livre. Fechar essa janela retorna à seleção livre. Os painéis usam o padrão visual de Mover e ficam ancorados abaixo do controle de vista, inclusive ao reabrir a ferramenta ou redimensionar a janela. As operações e restrições continuam abrindo seus próprios formulários, substituindo o painel da ferramenta anterior. Sobre o canvas, o ícone da variação escolhida acompanha o cursor, afastado de sua ponta; deixa de aparecer sobre painéis, campos e cubo de vista.

Os painéis **Linha**, **Retângulo**, **Círculo** e **Arco** oferecem **Traço → Normal / Referência**. Normal participa de áreas e extrusões; Referência aparece tracejada em laranja e serve como referência. **Infinita** aparece somente em Linha e Retângulo: cria retas suporte que atravessam o canvas, capturáveis além dos apoios, sem participar de áreas/extrusões. Em Retângulo, os quatro lados viram retas infinitas. Mudar Traço ou modo cancela a prévia e afeta as próximas entidades. X alterna Normal/Referência na seleção existente ou no traço da ferramenta. Duplo clique seleciona os elementos ligados por apoios e restrições dirigentes; Del exclui a seleção. Seleções são filtradas depois de desfazer/refazer para não manter IDs removidos.

- **Tipo de linha → Normal / Simétrica:** Normal desenha o segmento entre cliques. Simétrica acrescenta outro segmento de mesmo comprimento no lado oposto ao primeiro clique, que funciona como centro. Cada confirmação continua do último ponto; **Esc** encerra a sequência preservando os segmentos confirmados. Fechar uma área também encerra, inclusive ao cruzar um trecho anterior ou completar um contorno com linhas já existentes. Voltar sobre uma mesma reta não fecha área.
- **Tipo de retângulo → Normal / Do centro para fora / Inclinado:** Normal usa dois cantos opostos. Do centro para fora usa centro e canto, crescendo dos dois lados; os campos representam largura e altura totais. Inclinado usa três cliques: início do lado, fim do lado com comprimento/ângulo e largura perpendicular, escolhida pelo mouse ou digitada.
- **Tipo de círculo → Normal / Dois pontos (diâmetro) / Tangente:** Normal usa centro e raio. Dois pontos usa extremos do diâmetro e exibe seu segmento guia; a medida digitada é o diâmetro. No terceiro modo, selecione duas linhas perpendiculares diferentes, mova o mouse para definir quadrante e raio e confirme com clique ou Enter. Também é possível digitar o raio; as tangências usam as retas suporte, inclusive além dos extremos dos segmentos. Seleções inválidas são recusadas. **Esc** cancela a seleção ou a prévia.

A seção **Medidas** edita as mesmas expressões exibidas junto ao cursor e aceita valores antes do primeiro clique. Linha oferece comprimento e ângulo (somente ângulo em Infinita); retângulos normal/central oferecem largura e altura totais; inclinado oferece comprimento/ângulo do lado e depois largura perpendicular. Círculos oferecem raio ou diâmetro, conforme o modo. Arco oferece raio e ângulo inicial; após confirmar o início, o ângulo passa a representar abertura. Digitar trava a medida; limpar devolve o controle ao mouse. Tab alterna campos no painel ou no canvas e Enter confirma a etapa atual.

Simetria das linhas, ortogonalidade dos retângulos, tangências dos círculos e medidas digitadas são mantidas por restrições no recálculo, com desfazer/refazer e salvamento. O centro escolhido para construir um retângulo e os extremos usados para construir o diâmetro definem a geometria inicial; esses gestos não criam restrições adicionais de simetria central ou de incidência dos extremos do diâmetro.

A ferramenta **Arco** segue o sentido inicial do mouse depois do segundo clique e conserva a abertura ao atravessar os quadrantes; medidas digitadas mantêm esse sentido. **Imagem** carrega PNG/JPEG e recebe dois cantos no canvas. O painel edita largura/altura, posição no plano e transparência (0–100%, padrão 50%), com leitura da posição na peça. Selecionar pelo interior ou pela árvore permite arrastar e redimensionar pelos cantos; curvas e apoios reais têm precedência na seleção. OK publica os campos, Esc cancela a edição pendente, e arrastes confirmados entram no desfazer/refazer.

Os menus dos rascunhos oferecem editar, renomear e excluir, respeitando dependências do histórico. Pastas vazias continuam visíveis e oferecem criar elementos. Imagens aparecem em um nó próprio da peça, com olho, edição e exclusão. Nomes automáticos usam sequências persistentes por tipo, sem reaproveitar números por renomeação ou exclusão.

### Dimencionar

**Dimencionar** (grupo Modificar, ou **D** no rascunho) funciona como a Dimension do SolidCraft, sem painel: o ícone acompanha o cursor e o elemento sob ele fica realçado. Clique no que medir. Pontos (extremos, centros, pontos soltos) têm prioridade sobre as curvas próximas:

| Escolhas | Cota |
| --- | --- |
| Uma linha | Comprimento; se a linha não estiver alinhada a um eixo, o lugar do texto escolhe a medida (veja abaixo) |
| Um círculo completo / um arco | Diâmetro (Ø) / raio (R) |
| Dois pontos | Distância, horizontal ou vertical, pelo lugar do texto |
| Ponto e linha, em qualquer ordem | Distância do ponto à reta suporte |
| Duas linhas | Ângulo do setor onde fica o texto; paralelas medem a distância entre elas |
| Linha e círculo/arco, em qualquer ordem | Da reta até a borda, perpendicular à linha; se a linha corta o círculo, quanto a borda passa da reta |

Assim que as escolhas determinam a cota, a caixa de valor recebe o teclado com a medida atual selecionada: digitar a substitui. Valem números, vírgula decimal, unidades (`1 in`), contas e parâmetros da peça; sem unidade, comprimentos usam mm e ângulos, graus. Enquanto se digita, o texto segue o cursor e a caixa fica ao lado dele. **Enter** posiciona o texto ali, ou no lugar padrão se o cursor não se moveu, e confirma em uma ação de desfazer; sem digitar, a medida exata entra sem arredondamento. **Esc** cancela escolhas e valor e retorna a Selecionar. Após confirmar com Enter, Dimencionar continua ativa para a próxima medida.

Antes de digitar, um clique no vazio fixa o texto ali e mantém a caixa com o teclado; um clique em outro ponto, linha ou círculo transforma a cota (por exemplo, de comprimento em ângulo). **Tab** devolve as escolhas: o próximo clique no vazio posiciona o texto. Entre dois pontos ou numa linha fora dos eixos, o texto de frente para eles mede a distância entre eles; acima ou abaixo, a horizontal; ao lado, a vertical. Pontos alinhados a um eixo sempre medem a distância.

Uma cota que a geometria já fixada não comporta (os graus de liberdade não diminuem) entra como cota de referência: mede a geometria, aparece entre parênteses em cinza e não tem valor a editar. Valores inválidos (comprimento negativo, raio zero, ângulo fora de 0° a 180°) deixam a caixa vermelha e não confirmam.

Todas as cotas do rascunho são desenhadas como cotas: linhas de extensão desde a geometria medida, linha de cota com setas e o valor; raios e diâmetros com R/Ø; ângulos como arco entre as linhas. O texto fica na posição escolhida, relativa à geometria, e a acompanha quando ela se move. A janela Selecionar continua listando as cotas para excluir ou editar o valor.

Ferramentas de modificação abrem um painel com seleção, campos pertinentes e OK/Cancelar (Enter/Esc). Espelhar oferece seleção múltipla e campo separado para o eixo. O campo Perfil da extrusão mantém a seleção ativa para alternar várias áreas do mesmo rascunho; todas aparecem realçadas e entram na mesma confirmação. Contornos projetados são perfis elegíveis. Deslocamento seleciona a cadeia inteira por padrão, com opção para uma única aresta, e oferece Normal/Dupla/Simétrica; a distância muda ao pressionar e arrastar a ponta da seta e pode ser editada no campo flutuante. Dupla tem duas setas independentes, publicadas numa única ação. Arredondar e Chanfrar transformam todos os cantos entre os segmentos selecionados e também têm seta de medida. A extrusão usa o mesmo padrão de seta. A movimentação do rascunho ocorre pelo arraste direto de vértices ou arestas na seleção livre, com prévia resolvida em segundo plano; Esc cancela e soltar confirma. Mover e seu atalho M foram retirados desse contexto. Espelhar, Deslocamento, Arredondar e Chanfrar exibem linhas translúcidas calculadas em segundo plano. Validação e cancelamento preservam o documento; confirmar resolve o rascunho em segundo plano e publica pelo comando de edição, com desfazer/refazer.

No contexto do rascunho, os atalhos são L Linha, R Retângulo, C Círculo, A Arco, P Projetar, G Aparar (apagar trecho), S Deslocamento, O Arredondar, H Chanfrar, E Espelhar e D Dimencionar. A edição de texto preserva a digitação; X também funciona com as medidas flutuantes ativas. Fora do rascunho, S abre a busca de ferramentas.

Horizontal/vertical escolhe a orientação predominante da linha; Coincidente usa as pontas mais próximas (ou centros de círculos); Tangente usa uma linha e um arco com extremidade compartilhada. Esses são os recursos atuais do solver do 3dnator. A referência visual não implica paridade com todas as variantes de comandos do SolidCraft.

**Aparar** remove o trecho sob o clique, entre os vértices de encontro ou extremos vizinhos, em linhas, círculos e arcos. Passar o mouse realça em vermelho o trecho que será removido. Círculos atravessam 0°/360° sem corte artificial na costura. A atração a pontos não desloca o clique do aparador. A ação preserva as demais curvas e admite desfazer/refazer.

Os indicadores de ângulo reto, centro, ponto médio e tangência só aparecem durante o desenho ou no apoio selecionado. F alterna a força da junção selecionada, assim como o cadeado contextual.

Os encontros reais entre as curvas aparecem como vértices e atraem o desenho. São apoios derivados, sem fragmentar automaticamente as entidades ou suas cotas; ao aparar, os limites tornam-se extremos reais das partes restantes. Triângulos fechados oferecem o baricentro e retângulos oferecem o encontro das diagonais, inclusive na geometria de construção. Pontos médios de linhas e arcos continuam disponíveis; a metade do arco fica sobre a curva.

Durante o desenho, os apoios do modelo e dos outros rascunhos visíveis são projetados ortogonalmente no plano em edição. Isso inclui pontos médios das arestas, centros de faces triangulares/retangulares, centros de círculos/arcos e quadrantes, mesmo quando a geometria está em outra altura ou orientação. O destaque e o clique usam o ponto projetado; não é necessário criar uma entidade de projeção. A consulta respeita visibilidade, isolamento e bloqueio das ocorrências, é preparada em segundo plano e se atualiza ao alterar o modelo ou o plano, sem limitar as referências aos primeiros corpos/ocorrências.

**Novo rascunho** entra na seleção de suporte sem modificar o documento. O ícone da ferramenta acompanha o cursor sobre a área 3D e a barra de estado indica o que escolher; não há faixa de instruções sobre a área de edição. Clique num plano colorido da origem (XY/XZ/YZ), num plano de trabalho visível da peça em construção ou numa face plana do sólido, inclusive os triângulos nas quinas de um cubo chanfrado. Como o suporte captura um plano fixo, a face não precisa ter nome estável; projetos já salvos também são aceitos. Cancelar/Esc sai da seleção. Faces curvas e malhas não definem planos de rascunho; a seleção de faces exige OCCT. Os atalhos explícitos de XY/XZ/YZ continuam disponíveis na busca de ferramentas por S.

Passar o mouse destaca apenas o suporte apto que receberá o clique: o plano da origem tem prioridade quando cobre uma face na tela; entre planos sobrepostos, só o último desenhado é destacado. Nas faces do modelo, a consulta em segundo plano confirma a superfície plana no B-rep e realça todos os seus triângulos na posição da ocorrência, respeitando visibilidade, isolamento, bloqueio e seção. Faces curvas e malhas não recebem destaque nem permitem realçar um suporte oculto atrás delas. Mover o cursor, mudar a câmera ou a revisão impede reutilizar uma resposta antiga sobre outro alvo.

A edição acontece no mesmo viewport do modelo. A câmera se desloca e alinha ao plano, conservando a projeção; os sólidos continuam desenhados. Grade, entidades, cotas, encaixes e cursor usam o plano local da peça e sua posição na montagem. O cubo, pan, zoom e órbita continuam disponíveis durante a edição. Ao concluir, a câmera retorna à orientação anterior, mantendo o alvo e a escala de trabalho.

Depois do primeiro clique, medidas aparecem em campos flutuantes independentes, sem caixa de diálogo compartilhada. Na linha, o comprimento acompanha o mouse e o ângulo fica ao lado do arco de prévia tracejado preto. Os campos permanecem dentro do canvas e mantêm uma folga entre si, inclusive em linhas curtas:

| Forma | Campos |
| --- | --- |
| Linha Normal/Referência | Comprimento junto ao mouse; ângulo junto à prévia, relativo à linha capturada no primeiro ponto ou à horizontal quando o ponto está livre; sentido positivo anti-horário |
| Linha Infinita | Ângulo; o segundo clique define a direção, sem comprimento finito |
| Retângulo | Largura e altura positivas; o cursor escolhe o quadrante |
| Círculo | Raio |
| Arco | Raio e orientação inicial; após confirmar o início, raio e abertura anti-horária |

O primeiro campo recebe foco automaticamente. Digite a medida diretamente; **Tab** ou **Shift+Tab** alterna os campos, mantendo os valores já digitados. Um ponto azul identifica cada medida travada. Os outros campos continuam seguindo o cursor; apagar completamente um campo libera sua medida. São aceitas vírgula decimal, expressões, unidades e parâmetros da peça. Sem unidade explícita, comprimentos usam mm e ângulos usam graus; `0,5 rad` também é aceito. Comprimentos devem ser positivos e não exceder 1.000.000.000 mm; abertura de arco fica estritamente entre 0° e 360°.

**Enter** confirma a etapa com uma só tecla, inclusive durante a digitação. Um clique na área CAD também confirma, usando os valores travados. No arco, a primeira confirmação escolhe a ponta inicial e a segunda cria o arco; uma abertura digitada também fixa a orientação inicial para preservar o setor ao recalcular. **Esc** descarta todo o desenho em andamento e seus campos; a ferramenta continua pronta para uma nova forma. Valores inválidos aparecem junto aos campos e bloqueiam Enter/clique até serem corrigidos ou cancelados. Confirmar publica uma edição com desfazer/refazer; medidas digitadas recebem cotas dirigentes e expressões no documento. A captura automática de uma ponta é descartada quando ela não coincide com a ponta definida pelas medidas.

Ao desenhar perto de 90° em relação à linha de origem, a direção é atraída para a perpendicular. Aproximar a ponta de outra linha atrai o término para o pé da perpendicular; cruzá-la conserva a ponta além do encontro e ajusta a direção. A captura exige proximidade angular de 3° e correção de até oito pontos de tela, sem atrair referências distantes ou prolongamentos fictícios de segmentos finitos. **Alt** suspende a captura; digitar um ângulo tem prioridade sobre a atração. Uma perpendicular confirmada recebe sua restrição no documento, e a cota angular relativa acompanha mudanças da linha de origem.

A origem do projeto (global) aparece na área 3D por padrão; a origem de cada peça aparece quando o olho de **Origem** dentro da peça está aberto (fechado por padrão), em todas as ocorrências dela. Os planos coloridos só aparecem durante a seleção de suporte. Em uma face, a origem do rascunho é a projeção da origem da peça no plano, com eixos definidos pela normal local; clicar em outro ponto da mesma face não desloca essa referência. O suporte é capturado como plano fixo: ainda não há vínculo paramétrico com alterações posteriores da face.

### Plano deslocado

**Plano deslocado** (grupo Construção da aba Peça) cria um plano de trabalho na peça em construção; sem peça ativa, o botão fica indisponível. O painel abre com o campo **Plano** ativo: escolha um plano da origem (XY/XZ/YZ), um plano de trabalho visível da peça ou uma face plana do sólido, com o mesmo realce do Novo rascunho. Um plano de trabalho também pode ser escolhido pelo clique na árvore, inclusive quando oculto. Faces curvas ou cliques fora do modelo apenas avisam na barra de estado, mantendo a escolha aberta. Durante a escolha, o ícone do Plano deslocado acompanha o cursor. Esc ou Cancelar sem plano-base encerra a ferramenta; com plano-base, Esc só sai da escolha.

Escolhido o plano-base, aparecem o contorno tracejado dele, o plano novo em âmbar e uma seta com a distância. Arraste a alça para mover o plano ao longo da normal do plano-base; com o plano de frente para a câmera, o arraste é vertical. **Distância (mm)** aceita digitação com vírgula decimal, unidades e contas; o botão ao lado inverte o sentido. A distância inicial é 10 mm, e valores negativos vão para o lado oposto à normal. **OK** ou Enter cria o plano (“Plano 1”, “Plano 2”…) em uma ação de desfazer; nada muda no documento antes disso.

O plano-base é uma captura fixa nas coordenadas da peça, como os rascunhos sobre faces: alterações posteriores da face ou do plano de origem não movem o plano de trabalho. Ele aparece na área 3D como um retângulo âmbar centrado na projeção do centro do modelo e na pasta **Planos** da peça. Criar um rascunho nele, pela seleção de suporte do Novo rascunho ou pelo menu **Novo rascunho neste plano** da árvore, oculta o plano na mesma ação; o olho volta a mostrá-lo. O rascunho fica apoiado no plano.

Para mudar a distância de um plano existente, dê duplo clique nele na árvore ou use **Editar** no menu, inclusive com o plano oculto. O painel reabre com o nome do plano no título, **Plano-base atual** e a distância gravada; a seta e o campo funcionam como na criação, e a prévia substitui o plano na área 3D até o OK. O campo Plano permite escolher outro plano-base. OK sem mudanças só fecha o painel. Com mudanças, os rascunhos apoiados acompanham o plano, conservando a geometria em U/V, e as operações da peça são recalculadas em segundo plano, numa única ação de desfazer. Uma falha de recálculo abre a decisão de correção, supressão ou cancelamento usada nas demais edições. Excluir o plano mantém os rascunhos na última posição, sem o vínculo.

O rascunho permanece visível ao concluir. Criar uma extrusão, revolução ou furo oculta o rascunho usado, apenas depois de uma operação bem-sucedida. Cada rascunho tem um olho na árvore, inclusive em subpeças, para mostrar/ocultar sem abrir a edição. Clicar no nome abre a edição e mostra o perfil. A visibilidade é salva e participa de desfazer/refazer; recálculos não anulam uma reexibição manual. Projetos anteriores preservam a visibilidade já armazenada.

O rascunho existente é enquadrado ao abrir, considerando centros e extensões de círculos/arcos. O comando **Enquadrar**, disponível na busca por S, restaura essa vista após zoom, deslocamento ou redimensionamento da janela. As cotas usam a formatação em português, sem zeros decimais desnecessários.

A árvore usa ícones, controles de visibilidade, nomes abreviados por elipse e pastas por tipo (ver **Árvore do projeto**). Duplo clique em peça entra na edição dela. O ícone de pasta no canto da faixa recolhe/restaura a árvore. O painel lateral Propriedades foi eliminado; o viewport ocupa todo o espaço à direita da árvore. A árvore tem largura ajustável e rolagem; a faixa de ferramentas também rola horizontalmente em janelas menores.

Arraste o fundo livre da barra escura para mover a janela. Duplo clique nessa região maximiza ou restaura; as bordas e os cantos redimensionam a janela quando ela não está maximizada. Os botões da barra recebem seus próprios cliques. Fechar o aplicativo aguarda a tarefa em andamento e segue a confirmação de salvamento dos projetos modificados.

## Linha do tempo do histórico

Na base há uma única barra: **Histórico**, desfazer/refazer, um separador, a linha do tempo das operações da peça ativa e, à direita, um ponto colorido seguido do estado ou da mensagem. A linha do tempo representa o histórico paramétrico persistido; desfazer/refazer pertence à sessão e cobre todas as ações abaixo. A faixa central rola horizontalmente e não invade a mensagem. O rodapé não mostra a unidade.

Mensagens informativas e avisos aparecem na barra de status, com ponto verde ou laranja, sem interromper o trabalho. A mensagem permanece após a preparação da cena; durante o processamento, a barra indica a atividade. Mensagens longas recebem mais espaço, limitado a 45% do rodapé, e são abreviadas em uma linha, com o texto completo na dica do mouse.

Somente erros abrem diálogo, com OK, Enter ou Esc para confirmar a leitura. A fila preserva erros recebidos no mesmo quadro; informações e avisos não entram nessa fila. Falhas paramétricas mantêm o diálogo específico com as opções de resolução, sem repetir o mesmo diagnóstico em outra janela.

- **Reprodução:** os botões ⏮ ◀ ▶ ⏭ movem o marcador do histórico para o início, uma operação para trás, uma para a frente ou para o fim.
- **Marcador:** a barra azul com alça pode ser arrastada entre as operações para voltar a uma etapa. As operações depois dele ficam esmaecidas e não são construídas; seus resultados ficam ocultos. Novas operações entram na posição do marcador, que avança junto. Avançar o marcador reconstrói as operações reativadas.
- **Reordenar:** arraste uma operação para outra posição. A ordem só muda se tudo o que ela usa continuar antes dela e tudo o que usa seu resultado continuar depois; caso contrário, uma mensagem nomeia a dependência.
- **Menu de contexto (botão direito):** editar operação, editar o rascunho do perfil, renomear, suprimir/reativar, mover o marcador para depois da operação e excluir. Duplo clique edita; Delete exclui a operação destacada.
- **Estados:** operações suprimidas aparecem riscadas em vermelho; as que dependem de uma suprimida ficam acinzentadas, sem construir; as posteriores ao marcador ficam veladas. Suprimir não recalcula o restante; reativar reconstrói o que volta ao histórico.
- **Excluir com dependentes:** se outras operações usam o resultado, um diálogo lista todas e oferece excluí-las juntas ou cancelar, com a prévia do resultado na área 3D. Sem dependentes, a exclusão é imediata (e reversível por desfazer).

### Falha no recálculo

Quando uma edição (cota de rascunho, parâmetro, operação, reordenação ou reativação) faz uma operação falhar, nada é aplicado e o diálogo **Falha no recálculo** mostra a operação, a mensagem do núcleo e as operações afetadas. Na linha do tempo, a operação que falhou fica em vermelho e as dependentes em laranja; o modelo exibido continua sendo o último resultado válido, anterior à edição pendente.

- **Editar a operação** abre o painel dela; ao aplicar, a correção e a edição pendente entram juntas, em uma só ação de desfazer.
- **Suprimir e aplicar** aplica a edição suprimindo a operação que falhou (e, por consequência, as dependentes).
- **Cancelar edição** descarta a edição. Uma alteração de rascunho pendente continua no editor de rascunho até ser corrigida ou cancelada.

Nenhuma referência perdida é substituída por outra face ou aresta: a decisão é sempre do usuário.

## Seleção na área 3D

- **Pré-seleção:** o elemento sob o cursor é realçado em azul claro antes do clique, com o nome numa etiqueta. Na peça em construção, o modo de seleção da janela Medir seleção decide entre face, aresta e vértice; fora dela, a unidade é o componente inteiro. Com um painel aberto, o campo ativo decide (arestas, faces ou corpos), e outras peças não são realçadas. A pré-seleção é desligada acima de 400 mil triângulos na cena.
- **Seleção:** corpos e componentes selecionados ficam tingidos de azul, com arestas vivas azuis, desenhados pela GPU com profundidade. Faces e arestas escolhidas e as referências do painel aparecem realçadas sobre o modelo.
- **Ctrl+clique** acrescenta ou retira um elemento, na área 3D e na árvore. Clicar no vazio limpa a seleção.
- **Janela:** arraste com o botão esquerdo a partir de um ponto livre. Da esquerda para a direita (contorno azul), entram só os elementos inteiramente contidos; da direita para a esquerda (tracejado verde), os tocados. Com Ctrl, a janela acrescenta à seleção. Na montagem, a unidade é a instância; na construção, o corpo da peça ativa.
- **Sobrepostos:** segure o botão esquerdo parado sobre o modelo para listar todos os elementos sob o cursor, do mais próximo ao mais distante. Percorrer a lista realça cada um; o clique seleciona (com Ctrl, alterna). Esc ou um clique fora fecha a lista.

No movimento **Vértice a vértice**, círculos e arcos de sólidos CAD oferecem um **centro virtual**, inclusive no vazio de um furo ou fora do material. Cada centro fica no plano da curva: numa arruela, há apoios nas alturas das duas superfícies, e a vista decide qual está à frente. Passar pelo disco circular ou pelo setor do arco revela o centro; aproximar o cursor a até 14 pontos de tela mostra a mira encaixada nele. O alcance de captura independe do zoom.

As arestas oferecem também o **ponto médio**, na metade do comprimento percorrido sobre a curva. Nos arcos, ele fica sobre a curva, enquanto o centro virtual fica no centro do círculo de apoio. Círculos completos oferecem quatro **quadrantes na borda**, espaçados a cada 90° no plano local. Vértices, pontos médios e quadrantes seguem a mesma captura por proximidade. O ponto destacado é o usado pelo clique, tanto na origem quanto no destino; rascunhos de referência continuam prioritários como destino e oferecem pontos, extremos, pontos médios de linhas/arcos e quadrantes de círculos. A captura respeita ocultação, bloqueio, isolamento, seção e material à frente. Corpos importados como STL oferecem vértices e pontos médios das arestas vivas, sem apoios nas diagonais internas de superfícies planas.

Faces triangulares e retangulares também oferecem o **centro da face** no movimento ponto a ponto. Os apoios ficam sobre a superfície, acompanhando corpo e ocorrência. Um furo assimétrico num retângulo CAD não desloca seu centro geométrico; rascunhos de referência oferecem também os centros dos polígonos e os vértices dos cruzamentos. Em malhas, triângulos coplanares são agrupados antes de reconhecer o contorno, evitando centros fictícios das diagonais internas.

## Importação de STL

O STL não informa a unidade. Depois da leitura, o painel **Inserir malha STL** mostra o nome, a quantidade de triângulos, as dimensões lidas e as dimensões resultantes em milímetros para a unidade escolhida (mm, cm, m, in ou um fator personalizado). OK insere o corpo de malha com a escala aplicada; Cancelar descarta a leitura. A malha continua sendo referência, sem conversão para sólido.

## Peças de outro projeto

**Peça de outro projeto** (busca de ferramentas por S) lista os outros projetos abertos em abas e suas definições, com as de topo primeiro e a quantidade de subpeças. OK copia a definição escolhida para o projeto ativo como peças novas e independentes: subpeças repetidas continuam compartilhando a definição copiada e os grupos internos vêm junto. Na construção, a cópia entra como subpeça da peça ativa; na montagem, na raiz. Não há vínculo com o arquivo de origem, que não é alterado. Para repetir uma peça do próprio projeto, use **Nova instância**.

## Unidades, precisão e miniatura

**Arquivo → Casas decimais exibidas** permite escolher de 0 a 6 casas para medidas, dimensões do corpo na janela Medir seleção, coordenadas e parâmetros. A escolha é uma configuração do documento, salva no projeto e reversível por desfazer; não muda a precisão interna. As cotas dos rascunhos continuam omitindo zeros desnecessários.

Cada gravação inclui uma miniatura isométrica do modelo, mostrada na janela de recuperação para reconhecer as cópias.

## Reparar rascunho

No editor de rascunho, **Reparar rascunho** abre um painel com as correções sugeridas pelo diagnóstico de perfis:

- **Remover entidade duplicada:** mesma linha, círculo ou arco que outra; a mais recente é removida.
- **Remover trecho sobreposto:** linha inteira sobre outra linha colinear que já a cobre.
- **Unir pontas abertas:** extremos soltos mais próximos que a **folga máxima** (0,5 mm por padrão) recebem uma restrição de coincidência; o solver fecha a folga respeitando as demais restrições.

Cada sugestão tem caixa de seleção e aparece em laranja na área 3D; cruzamentos aparecem como × vermelho e pedem **Aparar**, pois exigem escolher o trecho. **OK** aplica só as marcadas pelo mesmo caminho das demais edições (com desfazer); restrições e cotas que dependiam de uma entidade removida saem junto, e pontos isolados de furos são preservados. Nada é corrigido sem confirmação.

## Corte de seção

**Corte de seção** (aba Inspecionar) esconde a parte do modelo para onde aponta a seta do plano e mostra a seção como tampa hachurada. O plano pode ser XY, XZ, YZ ou o da face escolhida na área 3D (capturado no momento). O deslocamento é digitado ou ajustado arrastando a seta vermelha; **Inverter** troca o lado escondido. OK mantém o corte; Cancelar volta ao estado anterior.

O corte é um controle visual da aba, como isolamento e transparência: não altera geometria, histórico nem desfazer, e não é salvo no projeto. Aparece em **Análises** na árvore, com olho para mostrar/ocultar e menu para editar ou excluir. Seleção, pré-seleção e a lista de sobrepostos ignoram a parte cortada. A tampa é desenhada pelas faces internas do sólido: em malhas abertas, o interior exposto aparece igualmente hachurado.

## Árvore do projeto

A árvore usa linhas compactas e começa diretamente no contexto de edição ou na raiz, sem o cabeçalho “Árvore do projeto”. A raiz **Projeto**, com o ícone do aplicativo e sempre visível, contém **Peças**, **Rascunhos** (de referência da montagem) e **Origem**. Cada peça é um nó com o nome da ocorrência (a definição aparece na dica) e as pastas **Corpos**, **Sub-peças**, **Rascunhos**, **Planos** e **Origem**; cada subpeça repete a mesma estrutura. Pastas vazias permanecem visíveis para criar elementos pelo menu de contexto. O nó Imagens reúne as referências raster da peça.

- **Corpos e operações:** a pasta Corpos lista os resultados atuais, sem acumular etapas anteriores ocultas. Arredondar, chanfrar ou modificar um corpo conserva seu nome; a operação aparece na linha do tempo. Para inspecionar uma etapa anterior, recue o marcador do histórico. Corpos independentes ocultados manualmente continuam listados para permitir reexibi-los, assim como etapas antigas explicitamente mostradas em projetos existentes.
- **Visibilidade:** os descendentes da raiz têm um olho. O da pasta não altera a escolha de cada elemento: a visibilidade efetiva combina a do elemento com a dos nós acima dele, e o que fica oculto por um ancestral aparece esmaecido. **Peças** controla as peças da raiz; **Sub-peças**, as subpeças daquela peça. As opções são gravadas no projeto, participam de desfazer/refazer e valem para todas as ocorrências da peça. Uma raiz oculta em arquivo antigo é restaurada ao abrir, conservando as escolhas dos descendentes.
- **Origem:** a do projeto começa visível; a de cada peça começa oculta e, aberta, desenha os eixos em cada ocorrência da peça.
- **Planos:** planos de trabalho da peça, com olho próprio, deslocamento na dica e menu com **Editar**, **Novo rascunho neste plano** e **Excluir**; o duplo clique também edita. Durante uma seleção de suporte, clicar no plano o escolhe.
- **Edição da peça:** o lápis ao lado do olho entra na edição da peça (construção), abre o nó dela e leva a faixa da aba Montagem para Peça; na peça em edição, o lápis fica destacado e um novo clique volta à montagem (e a faixa, de Peça para Montagem). O nome da peça em edição fica no topo da árvore, com um botão para sair.
- Clique seleciona (Ctrl+clique acrescenta); clique num rascunho abre a edição dele; clicar no nome de uma pasta abre ou fecha a pasta. Grupos, análises e folhas técnicas ficam abaixo da raiz.

## Montagem e posicionamento

O menu **Mover** tem as mesmas quatro opções nas abas Peça e Montagem: **Livre**, **Face a face**, **Aresta a aresta** e **Vértice a vértice**. Na Peça, o elemento movido é um corpo da peça ativa; na Montagem, é um componente. Para alinhamentos na Peça, apenas a origem fica restrita à peça em edição: o destino pode pertencer a outra peça ou a um rascunho de referência.

- **Rascunho de referência:** escolha um plano da origem ou uma face (o plano é capturado no momento e não acompanha a face). O rascunho pertence à montagem, não a uma peça: é desenhado em violeta, fica na pasta **Rascunhos** do projeto, na árvore (olho, edição por clique, menu com editar e excluir) e nunca serve de perfil para extrusão, revolução ou outra operação. Serve de destino no Mover face a face (plano do rascunho), aresta a aresta (linhas) e vértice a vértice (pontos e extremos).
- **Nova peça:** **Criar nova peça** acrescenta uma peça vazia na raiz da montagem, já ativa para construção, e muda para a aba Peça. **Criar a partir de STEP** importa cada definição do arquivo como peça nova, com cores e hierarquia; exige o núcleo OCCT, e um executável compilado sem ele mostra o motivo no próprio item. **Criar a partir de STL** abre o painel de unidade e insere a malha como peça nova. Os diálogos aceitam extensões em maiúsculas (`.STEP`, `.STP`, `.STL`). As três desfazem em uma ação.
- **Posição ao lado:** peças novas (vazias, STEP, STL ou de outro projeto), novas instâncias e cópias independentes não aparecem sobrepostas: o componente desliza em +X do mundo até ficar a uma folga das ocorrências visíveis (um décimo da sua maior dimensão, entre 5 e 100 mm). As raízes de um STEP vão juntas, preservando o arranjo do arquivo; uma peça vazia posiciona a origem. Corpos acrescentados a uma peça em construção ficam onde foram lidos, e Mover com **Criar cópia** usa exatamente o deslocamento pedido.
- **Árvore:** cada componente mostra o olho de visibilidade, o lápis de edição e o nome da ocorrência, com a definição na dica. O botão direito abre o menu do componente: ativar, Mover/Copiar, nova instância, clone independente, padrões, espelhar, inserir dentro de outra peça, agrupar a seleção, isolar, transparente, selecionável, ocultar/mostrar, renomear a ocorrência ou a definição e excluir. Corpos têm Mover/Copiar, ocultar/mostrar, exportar STL, cor do corpo (incluindo restaurar a cor neutra), renomear e excluir. A seção **Grupos** lista os grupos: o clique seleciona os membros; o menu move, renomeia ou desagrupa.
- **Mover:** o campo Objetos recebe instâncias (montagem) ou corpos da peça ativa (construção), a partir da seleção ou de cliques. Com o campo inativo, o clique escolhe um ponto (vértice, aresta ou face, pelo modo de seleção) que pode virar o pivô. O manipulador aparece no pivô: setas transladam por eixo, anéis giram em passos de 1°, com fixação nos múltiplos de 45° (0°, 45°, 90°, 135°…) a menos de 4°, e o centro move livremente no plano da tela; durante o arraste, o contorno dos elementos acompanha o cursor e a prévia completa é calculada ao soltar. Durante o giro, o ângulo aparece junto ao cursor, na cor do eixo quando fixado. Os campos de translação (mm) e rotação (graus), nos eixos do mundo e com rotação X→Y→Z pelo pivô, são digitáveis: o clique seleciona o valor para substituição e aceitam vírgula ou ponto, unidades e contas (`12,5`, `1 in`, `90 + 45`); texto inválido permanece destacado em vermelho mesmo fora do foco e bloqueia OK/Enter até a correção. Valores fora de ±10.000.000 são recusados sem truncamento. Esc no campo restaura o último valor válido sem fechar o painel; Zerar limpa também os textos pendentes. A origem do padrão circular usa as mesmas regras. O pivô é o centro de massa, a origem do primeiro elemento ou o ponto escolhido. **Criar cópia** gera novas instâncias da mesma definição ou corpos independentes, sem histórico. Vários elementos (por exemplo, todos os membros de um grupo) movem-se em uma só ação de desfazer.
- **Mover face a face, aresta a aresta e vértice a vértice:** abrem o painel de alinhamento já com o tipo escolhido (que ainda pode ser trocado). Clique na geometria do elemento que se move (destacada em azul) e depois na de destino (em violeta), que pode estar em outro componente ou num rascunho de referência; para pontos e arestas, as referências próximas do cursor têm prioridade sobre o modelo. Faces ficam opostas e coplanares no ponto clicado; arestas, paralelas; pontos, coincidentes. **Inverter direção** troca o sentido. Não cria restrição permanente.
- **Copiar:** **Nova instância** repete a definição ao lado da original (editar uma altera todas); **Cópia independente** cria, também ao lado, uma peça nova com histórico próprio; **Padrão de repetição** abre o painel de padrão retangular ou circular.
- **Padrões e espelhamento de componentes:** painéis com o componente, tipo, direção ou eixo, quantidade e espaçamento ou ângulo; o padrão compartilha a definição e reúne as instâncias num grupo. O espelhamento cria uma peça nova e independente.

## Painéis de comando

Todos os painéis de opções usam o padrão visual de **Mover** da aba Montagem: cabeçalho claro com ícone, nome em maiúsculas e fechar, bordas arredondadas, sombra e conteúdo com espaçamento uniforme. Ficam ancorados à direita da área 3D, 12 pontos abaixo do controle de vista inteiro (cubo, setas, Home e Enquadrar). A posição acompanha o canvas e é reaplicada ao reabrir; posições antigas não são restauradas. O conteúdo longo rola na altura restante, mantendo cabeçalho e rodapé acessíveis. A largura padrão é 330 pontos; tabelas de parâmetros e folhas técnicas mantêm espaço maior, limitado à largura disponível do canvas.

Operações de construção, montagem, reparo e modificação de rascunho mantêm o rodapé com estado da prévia, **OK** e **Cancelar**. Opções aplicadas durante o desenho, medições, parâmetros, folha e exportação usam seus próprios controles dentro do mesmo painel. A renomeação de operações também segue o padrão. Enter confirma e Esc cancela; dentro de um campo em edição, ambos apenas encerram a digitação (Enter conclui o valor), e o próximo Enter confirma o painel com o valor digitado.

- **Campos de seleção** (Perfil, Arestas, Faces, Corpo, Corpo alvo, Corpo ferramenta, Face limite) mostram a quantidade escolhida e um × para limpar. O campo ativo, em azul, recebe os cliques no modelo. Arestas e faces alternam: clicar de novo numa já escolhida a retira. Clicar em outro corpo recomeça a seleção.
- **Perfil na área 3D:** com o campo Perfil ativo, áreas fechadas dos rascunhos visíveis aparecem levemente sombreadas; a menor área sob o cursor é realçada e o clique a escolhe. Discos internos, lentes entre círculos, interseções e bordas compartilhadas são selecionáveis; ramos abertos não impedem usar outra área fechada nem são fechados implicitamente. Durante a edição de uma operação, seu rascunho oculto também fica disponível. O reconhecimento roda em segundo plano, com cache por aba/revisão.
- **Extrusão:** pode partir de uma área fechada do rascunho ou de uma face plana de um sólido da peça. Uma face já selecionada pode abrir diretamente como perfil; furos do contorno são mantidos. Depois de escolher o perfil, arraste a seta para definir a distância ou clique numa face plana da mesma peça para mudar automaticamente para Até face. O limite acompanha a referência gravada e recorta corretamente perfis diante de uma face inclinada. Faces paralelas à direção não servem de limite. Continuam disponíveis um lado, dois lados, simétrica, inversão, deslocamento inicial e medidas por expressão. Vistas de frente para o perfil usam arraste vertical para a distância. Soltar o manipulador mantém o painel aberto; OK confirma a operação.
- **Operação automática:** ligada por padrão nas novas extrusões. Percurso em espaço livre sugere Novo corpo; contato inicial com um corpo, saindo para fora, sugere Unir; entrada no material sugere Remover, inclusive quando um rascunho afastado alcança o corpo depois de atravessar espaço livre. Havendo mais de um corpo em contato, a origem explícita da face e o corpo selecionado têm prioridade. Escolher uma operação pelos botões desliga a sugestão; o corpo alvo continua visível no formulário e pode ser escolhido manualmente. Editar o histórico mantém a escolha gravada, sem reclassificá-la implicitamente.
- **Demais ferramentas:** revolução (eixo do plano ou linha do rascunho, ângulo ou volta completa), furo (tipo, extensão, ponta, rosca registrada), arredondamento, chanfro (distância igual ou distância e ângulo), casca (para dentro ou para fora), combinar (unir, cortar, interseção), padrões e espelhamento/divisão por plano. O índice topológico manual fica em **Avançado**.
- **Prévia ao vivo:** a cada alteração, depois de uma breve pausa, o resultado é calculado em segundo plano numa cópia do documento e mostrado no lugar da cena. Resultados ficam azuis; na remoção por extrusão, o corpo restante mantém sua cor e o volume efetivamente retirado fica vermelho translúcido. Trechos do prisma que passam pelo vazio não ficam vermelhos. Ao editar uma operação, a prévia mostra o histórico até ela, e as seleções valem para o modelo antes dela. Erros aparecem no rodapé. **OK** publica o resultado já calculado sem recalcular; se a prévia falhou numa operação posterior, abre-se a decisão de falha no recálculo.
- Seleções obrigatórias vazias desabilitam o OK. A prévia não entra no desfazer nem altera o documento.
- Cancelar ou fechar o painel descarta a prévia e restaura automaticamente a cena do documento, sem precisar clicar ou navegar no visualizador. Esc fora dos campos tem o mesmo efeito; Esc dentro de um campo numérico restaura primeiro o valor em edição.

## Busca, ícones e acessibilidade

**S** abre a busca de ferramentas; nomes sem acento encontram os rótulos portugueses. Enter executa o primeiro resultado habilitado e Esc fecha a busca. Os atalhos existentes Ctrl+S, Ctrl+Z e Ctrl+Shift+Z/Ctrl+Y foram mantidos. Os campos de texto conservam a entrada normal de letras.

Os ícones são vetoriais, desenhados em uma grade de 24 unidades pelo egui, sem fonte de símbolos ou dependência de imagens externas. Nomes completos ficam nos tooltips e na semântica de acessibilidade, incluindo botões sem texto. Botões aceitam foco e ativação por teclado; ferramentas indisponíveis respeitam o estado de processamento, o contexto da peça e a disponibilidade do núcleo.

O fundo claro do renderizador e do rascunho usa a mesma paleta dos painéis. Cores de seleção, erro, aviso, restrição e medição foram ajustadas para esse contraste. O esmaecimento das peças de referência usa a cor efetiva do fundo, preservando a transparência.

## Cubo de orientação

No canto superior direito da área 3D, o cubo tem azuis claros, chanfros largos, Home de contorno à esquerda, Enquadrar logo abaixo e duas setas curvas preenchidas. O controle mede 80% do desenho anterior (166,4 × 160 pontos de tela). Clique em uma face, aresta chanfrada ou canto para escolher uma das 26 direções. A área selecionável acompanha o polígono visível; os nomes em português aparecem nas faces e acompanham sua orientação, incluindo Frente, Superior e Inferior. A área de edição não tem a antiga barra permanente de vistas/projeção.

Arraste com o botão esquerdo sobre o cubo para orbitar livremente. O movimento continua ao passar pelas vistas superior e inferior; não seleciona nem desloca corpos. A órbita com Shift + botão do meio usa o mesmo referencial da tela.

O botão **Home**, à esquerda do cubo, restaura a vista isométrica padrão, zera a inclinação da tela e enquadra o modelo, conservando a projeção ortográfica/perspectiva. **Enquadrar**, abaixo dele, usa o contexto do editor: modelo na vista 3D ou rascunho em seu plano de trabalho.

As duas setas giram de **45° em 45°**, em sentidos opostos, ao redor do eixo perpendicular à face principal mais voltada ao observador. Em uma sequência de cliques, o eixo inicial é conservado mesmo se outra face passar a ocupar mais espaço; escolher uma face ou arrastar inicia uma nova referência. Nas vistas frontal, superior e inferior, o giro permanece no plano exibido. **Shift+clique** ou botão direito na seta inverte o sentido. Os controles não selecionam o modelo nem modificam alvo, zoom ou projeção.

O **botão direito no cubo** abre o menu com **Ortográfica**, **Perspectiva**, **Enquadrar** e os mesmos giros de 45°. Vistas convencionais e enquadramento também continuam na busca de ferramentas por S.

O controle acompanha a câmera de cada aba durante a sessão. Ele atua na visualização; não acrescenta operações ao histórico paramétrico ou ao desfazer/refazer do documento.

## Código e validação visual

- `apps/prototype/src/desktop/project_editor/theme.rs`: paleta, tipografia e medidas.
- `icons.rs`: desenhos originais e widgets de ícone/linha de árvore.
- `shell.rs`: barra superior, faixa por tarefa, busca e histórico visual.
- `window_frame.rs`: controles da janela, arraste e redimensionamento sem decoração nativa.
- `view_cube.rs`: geometria de apresentação, seleção e interação do navegador de orientação, com pintura recortada.
- `views.rs` e `navigation.rs`: painéis e navegação do viewport CAD único.
- `project_tree.rs`: árvore do projeto (pastas, visibilidade efetiva, lápis de edição); `browser.rs`: menus de contexto e renomeação.
- `sketch_editor.rs`: ferramentas, restrições e publicação das edições do rascunho.
- `sketch_scene.rs`: origem, seleção de plano, alinhamento da câmera e projeção dos rascunhos no viewport.
- `crates/renderer/src/lib.rs` e `view.wgsl`: cor de apresentação do fundo, incluindo o esmaecimento.

A captura do projeto mostra a modelagem 3D. Para revisar o rascunho aberto sobre o modelo no mesmo viewport, use a captura específica de rascunho. Os dois comandos aceitam dimensões opcionais (900–3840 × 600–2160 pixels):

Estados interativos também podem ser capturados por cenários (`timeline`, `timeline-menu`, `extrude`, `edit-last`, `failure`, `hover-face`, `hover-edge`, `select-body`, `window`, `assembly-select`, `move`, `align`, `assembly-ribbon`, `section`, `repair`, `recovery`, `insert`, `mesh-import`, `menu-new-part`, `menu-move`, `menu-copy`, `reference-align`, `reference-sketch`, `copy-beside`, `tree-folders`, `part-menu-move`, `measure`, `sketch-line-input`, `sketch-circle-input`, `sketch-rectangle-input`, `extrude-face`, `extrude-cut`, `extrude-zero`, `paint`), que não alteram o projeto de origem:

```sh
cargo run -p nator-prototype --features occt -- --render-ui-scenario projeto.3dnator painel.png extrude
cargo run -p nator-prototype --features occt -- --render-project-preview projeto.3dnator modelo.png
cargo run -p nator-prototype --features occt -- --render-sketch-preview projeto.3dnator rascunho.png
cargo run -p nator-prototype --features occt -- --render-project-preview projeto.3dnator compacto.png 900 600
```

As capturas usam a mesma composição egui/wgpu da janela. Elas verificam layout e renderização; não substituem a interação com diálogos nativos. Testes de eventos verificam clique, bloqueio de botões desabilitados, nome acessível e ativação por teclado, além da busca em português sem acento.

A validação Windows está adiada por orientação do usuário. O workflow a mantém disponível por acionamento manual com `validate_windows`, enquanto o trabalho atual prioriza Linux e refinamento funcional.

O ensaio de interação do rascunho usa eventos egui e um adaptador wgpu (físico ou software), sem janela nativa. Ele é optativo no conjunto de testes:

```sh
cargo test -p nator-prototype --features occt sketch_workflow_keeps -- --ignored
```
