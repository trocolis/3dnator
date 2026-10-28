# Histórico de mudanças

## Não lançado — 0.3.0 em desenvolvimento

### Refinamento das ferramentas de rascunho

- Mover no rascunho por deslocamento X/Y ou dois pontos, com resolução obrigatória da posição solicitada e erro quando as restrições impedem o movimento. Duplo clique seleciona os elementos ligados por apoios e relações dirigentes; Del exclui a seleção e desfazer limpa alvos que deixaram de existir.
- Modificar reúne Aparar, Deslocamento, Arredondar, Chanfrar, Espelhar, Mover e Dimencionar. Padrão e Restringir ficam temporariamente ocultos, inclusive na busca, preservando as restrições dos projetos existentes.
- Deslocamento de cadeias e contornos completos de linhas/arcos, com encontro dos apoios deslocados nos cantos. Escolha da distância pelo mouse, campo flutuante editável e confirmação dos dois lados no modo Dupla. Espelhar, Deslocamento, Arredondar e Chanfrar apresentam prévias translúcidas calculadas em segundo plano; cancelar não modifica o documento.
- Traço Construção passa a se chamar Referência. X alterna Normal/Referência nas entidades selecionadas ou nas próximas entidades da ferramenta. Atalhos locais do rascunho: L/R/C/A/P/G/S/O/H/E/M/D; S continua abrindo a busca fora desse contexto.
- Símbolo de centro de círculo somente com vínculo ao centro; extensões de cotas partem do vértice mais próximo do texto, sem percorrer toda a linha de referência. Os 16 novos itens do AJUSTE.md foram concluídos. Aplicativo 0.3.0 em desenvolvimento e formato interno 25 preservados.

### Operações de rascunho e dimensionamento por conexões

- Cotas existentes são selecionadas pelo texto, arrastadas para reposicionar e editadas por duplo clique. Enter aplica a expressão e Esc cancela. A edição conserva ID, posição do texto e ordem dos elementos; arrastar modifica somente a apresentação e cria uma ação de desfazer ao soltar. Cotas de referência continuam medindo a geometria, com a opção de referência disponível no painel.
- Painel Espelhar com campos independentes Entidades e Eixo de espelhamento; cliques no desenho alimentam o campo ativo. Painel Deslocamento com modos Normal/Dupla/Simétrica e duas expressões independentes no modo Dupla. As duas ferramentas oferecem Normal/Construção/Infinita para as cópias; o traço infinito exige linhas.

- Política de cotas centralizada no núcleo: o elemento mais conectado é preservado; no empate, o segundo escolhido fica no lugar e o primeiro se ajusta. Ligações fracas não contam como apoios, e a própria cota é excluída da contagem. Restrições existentes prevalecem sobre a preferência transitória.
- Espelhamento de múltiplas entidades com eixo separado e traço Normal/Construção/Infinita, preservando apoios compartilhados e coincidências internas nas cópias. Deslocamento Normal, Duplo e Simétrico, com distâncias por lado e o mesmo seletor de traço. Arcos passam a aceitar deslocamento radial. Falha em qualquer entidade/lado descarta a operação inteira.
- Indicadores geométricos calculados fora da interface para perpendicularidade, tangência, ponto médio e centro. Formato interno 25 registra a ordem dos dois elementos cotados em `dimension_targets`, lendo os formatos anteriores. Aplicativo permanece em 0.3.0 em desenvolvimento.
- Indicadores pequenos junto à geometria, com dicas ao passar o mouse e posições separadas do cadeado. Centros de círculos/arcos e polígonos, pontos médios e relações de tangência/perpendicularidade aparecem também durante as capturas/prévias correspondentes. Checklist AJUSTE.md concluído.

### Junções fortes/fracas e cadeados nos vértices

- Captura em vértices cria ligação forte por padrão; cruzamentos e pontos no interior de curvas criam ligações fracas. **F** inverte a decisão do próximo clique durante o desenho, inclusive com as medidas flutuantes ativas. A prévia mostra cadeado e força selecionada; o clique grava essa decisão, e o próximo apoio volta ao padrão.
- Selecionar um vértice com junção mostra um cadeado ao lado: fechado para fortalecer a ligação fraca e aberto para liberar a forte. Junções fortes têm ponto/anel verde-azulado; fracas têm círculo vazio. Abrir um vértice compartilhado separa os apoios sem deslocar a geometria. Cada alteração usa o comando do rascunho, com recálculo, desfazer/refazer e persistência.
- Ligações fracas conservam seus alvos como restrições geométricas de referência, sem obrigar a geometria a acompanhá-los. Cotas e demais restrições das entidades continuam valendo. Uma tangência que exige extremo compartilhado é liberada junto desse extremo ao separá-lo. Cruzamentos fortes usam apoio de incidência sem partir as linhas; ao abrir um apoio sem cotas próprias, ele volta a ser virtual.
- Formato interno 24 acrescenta `PointOnCurve`, para que ligações à borda acompanhem o raio de círculos/arcos, e documenta `reference` para ligações geométricas fracas. Projetos anteriores continuam legíveis; aplicativo permanece em 0.3.0 em desenvolvimento.

### Dimencionar e ordem dos elementos

- A ferramenta Cota passa a se chamar **Dimencionar** na faixa, na busca e no painel, mantendo o atalho D.
- Ao dimensionar dois elementos livres, o primeiro escolhido permanece no lugar e o segundo se ajusta. A preferência considera todos os apoios do elemento e o raio dos círculos, sem gravar travas adicionais; restrições existentes continuam prevalecendo quando o segundo elemento já está fixado.

### Projetar componentes no rascunho

- Nova ferramenta **Projetar** no grupo Criar da aba Rascunho e na busca, com filtros para faces planas, arestas/arcos e vértices. A prévia realça a referência sob o cursor; um clique captura sua projeção ortogonal no plano local, respeitando as posições do corpo, da ocorrência e da peça editada. Fontes ocultas ou bloqueadas não são oferecidas.
- Referências persistentes com traço lilás e preenchimento lilás claro nos contornos fechados, preservando furos. Pontos e raios capturados ficam fixos, disponíveis para captura, cotas e perfis; repetir a projeção não duplica a geometria. Cada captura oferece desfazer/refazer; Esc encerra a ferramenta.
- A consulta do B-rep e o cálculo das áreas rodam fora da interface. Círculos e arcos paralelos ao plano conservam sua geometria exata; curvas inclinadas e outras curvas usam polilinhas com desvio máximo de 0,01 mm. Malhas fornecem vértices e arestas vivas, sem diagonais de triangulação. As referências são capturas fixas, sem vínculo de atualização com o componente.
- Formato interno 23 grava `projected` nos pontos e entidades, lendo projetos anteriores sem essas marcas. Referências fixas não consomem o limite de variáveis livres do solver. Mantida a versão 0.3.0 em desenvolvimento.

### Selecionar nas abas e alvos pela seleção atual

- Selecionar passa a ser o primeiro item de Montagem, Peça e Rascunho, com ícone de cursor e destaque enquanto o modo está ativo. Seu ícone não acompanha o ponteiro na área de edição. Esc, ao cancelar uma ferramenta, retorna à seleção; prévias e gestos pendentes são descartados, preservando o documento e os elementos já selecionados.
- Ctrl+clique acrescenta ou retira elementos na área 3D, na árvore e no rascunho. Ctrl no vazio conserva os elementos selecionados; a seleção por janela continua aditiva com Ctrl. Mover recebe todos os alvos selecionados e aceita Ctrl para ampliar os objetos mesmo fora do campo de seleção.
- Ferramentas compatíveis aproveitam a seleção existente: origem de alinhamento pelo último clique válido, corpos de operações e padrões, dois corpos para combinar, face para casca/extrusão e suporte de rascunhos/planos, aresta de arredondamento/chanfro e entidades para Cota/restrições. Seleções incompatíveis continuam aguardando escolhas, e cliques de elementos retirados da seleção não são reutilizados.
- A sugestão de aresta é calculada em segundo plano. Trocar de ferramenta encerra a prévia anterior antes de assumir a interação; a seleção é preservada. Esc nos campos numéricos conserva primeiro a edição do valor, como nos painéis existentes.

### Cota no modelo da Dimension do SolidCraft

- A Cota do rascunho deixa de ser um formulário: com a ferramenta ativa (botão ou **D**), clique no que medir. Uma linha mede o comprimento; um círculo, o diâmetro; um arco, o raio; dois pontos, a distância; ponto e linha, a distância à reta; duas linhas, o ângulo (ou a distância, se paralelas); linha e círculo, a folga até a borda. Pontos têm prioridade sobre as curvas próximas, e o elemento sob o cursor fica realçado.
- Assim que a cota fica determinada, a caixa de valor recebe o teclado com a medida selecionada; digitar a substitui, com unidades, contas e parâmetros. O texto segue o cursor e Enter o posiciona e confirma em uma ação de desfazer; Esc cancela. Antes de digitar, um clique no vazio fixa o texto e um clique em outro elemento transforma a cota; Tab devolve as escolhas para um clique de posicionamento. A ferramenta continua ativa para a próxima cota.
- Entre dois pontos ou numa linha inclinada, o lugar do texto escolhe a distância, a horizontal ou a vertical. Sem digitar, a medida exata é mantida. Uma cota que a geometria já fixada não comporta entra como referência, entre parênteses.
- Todas as cotas são desenhadas como cotas: linhas de extensão, linha de cota com setas, valor, R/Ø nos raios e diâmetros e arco nos ângulos. O texto fica onde foi posicionado e acompanha a geometria.
- Novas restrições de diâmetro, distância ponto–linha e linha–círculo no solver; o ângulo entre linhas guarda o lado de cada linha, valendo exatamente o setor cotado. Formato interno 22 grava esses tipos e a posição do texto das cotas (`dimension_labels`), lendo os projetos anteriores.

### Barra superior, faixa e escolha de plano

- Barra de título com o nome 3Dnator (também no título da janela e no filtro de arquivos), Novo e Salvar logo após Arquivo e sem a busca. Ícones da barra escura com traço claro, destaque azul-claro, preenchimento escuro e realce de passagem escuro, legíveis inclusive ao passar o mouse.
- Busca de ferramentas à direita da linha das abas, no lugar do botão de mostrar/ocultar a árvore, que continua disponível na busca (S).
- Plano deslocado passa a ser o último grupo da aba Peça.
- Novo rascunho e Plano deslocado não mostram mais a faixa de instruções e botões XY/XZ/YZ/Cancelar sobre a área 3D. O ícone da ferramenta acompanha o cursor durante a escolha, como nas ferramentas de desenho; a barra de estado indica o que escolher e Esc cancela.

### Edição da distância dos planos de trabalho

- Duplo clique num plano da pasta Planos, ou **Editar** no menu, reabre o painel do Plano deslocado com o nome do plano, o plano-base atual e a distância gravada, mesmo com o plano oculto. Seta, campo e inversão funcionam como na criação; o campo Plano também permite trocar o plano-base. A prévia ocupa o lugar do plano gravado até o OK; OK sem mudanças não cria ação no desfazer.
- Rascunhos criados num plano de trabalho ficam apoiados nele. Mudar a distância move esses rascunhos, preservando a geometria local, e recalcula as operações da peça em segundo plano, numa única ação de desfazer; falhas de recálculo abrem a decisão de correção, supressão ou cancelamento. Excluir o plano mantém os rascunhos na última posição, sem o vínculo; clonar a peça leva os vínculos para os planos copiados.
- Formato interno 21 grava o apoio do rascunho (`support`), lendo os projetos anteriores com rascunhos de plano fixo.

### Plano deslocado e planos de trabalho

- Nova ferramenta **Plano deslocado** no grupo Construção da aba Peça. O painel escolhe o plano-base entre os planos da origem, os planos de trabalho da peça (na área 3D ou na árvore) e faces planas do sólido, com o realce do Novo rascunho. A seta arrasta o plano ao longo da normal; Distância aceita digitação com unidades e contas e pode ser invertida. OK cria “Plano N” em uma ação de desfazer; Esc/Cancelar não alteram o documento.
- Planos de trabalho aparecem em âmbar na área 3D e na nova pasta **Planos** da peça, com olho próprio, dica do deslocamento e menu **Novo rascunho neste plano**/**Excluir**. A seleção de suporte do Novo rascunho os oferece junto a XY/XZ/YZ. Criar um rascunho no plano o oculta na mesma ação; excluir o plano não move nem apaga o rascunho.
- Faces curvas ou cliques fora do modelo durante a escolha do plano-base apenas avisam na barra de estado. A ferramenta exige peça em construção e se encerra quando outra ferramenta assume a área 3D.
- Formato interno 20 grava `work_planes` e a visibilidade da pasta (`folders.planes`), lendo os projetos anteriores sem planos e com a pasta visível. Clonar, espelhar ou inserir uma peça copia seus planos com novos identificadores; como os rascunhos, eles não são refletidos pelo espelhamento.

### Faixas de ferramentas uniformes e compactas

- Montagem, Peça, Inspecionar e Rascunho compartilham a mesma composição e altura de grupos. Menus e rótulos simples reservam o mesmo rodapé, corrigindo a faixa mais alta no Rascunho.
- Botões da faixa com 72 × 54 pontos, ícones de 24 pontos, legendas de até duas linhas e margens/intervalos verticais menores. A galeria de restrições conserva as duas fileiras dentro da altura comum; os botões dos painéis de opções mantêm suas dimensões.
- A troca de aba não desloca o início do canvas, inclusive em janelas compactas com rolagem horizontal. Preservados menus, dicas, acessibilidade e as opções Concluir/Cancelar.

### Sair do rascunho com Concluir e Cancelar

- Grupo Sair integrado à mesma faixa rolável dos demais grupos, com botões Concluir/Cancelar e menu de opções no rótulo. Removido o espaço isolado que mantinha somente Concluir na extremidade direita.
- Concluir conserva o desenho. Cancelar descarta a prévia e restaura o conteúdo do rascunho como estava ao entrar na edição, inclusive quando há uma alteração pendente. A restauração passa por comando em segundo plano, recalcula dependências e admite desfazer/refazer, preservando outras alterações do projeto. Sem mudanças publicadas, cancelar não cria uma entrada no histórico.

### Painéis de ferramentas no padrão de Mover

- Cabeçalho com ícone, título em maiúsculas e fechar, espaçamento, bordas arredondadas e sombra do painel Mover passam a ser o padrão dos painéis de opções. Rascunho, medição, parâmetros, folha técnica, exportação e renomeação de operações usam o mesmo componente das ferramentas de montagem e construção.
- Painéis ancorados à direita, sempre abaixo do controle de vista completo. Reabrir ou redimensionar a janela reposiciona o painel, sem reaproveitar posições antigas. Conteúdo longo rola dentro da altura disponível, preservando o cabeçalho e o rodapé de confirmação.
- Opções de desenho conservam ícones, Medidas e confirmação pelo canvas; ferramentas que exigem confirmação mantêm estado da prévia, OK e Cancelar.

### Modos de construção e Traço nas ferramentas de desenho

- Linha com modos Normal e Simétrica: a segunda opção acrescenta um segmento de mesmo tamanho no lado oposto ao primeiro clique, mantendo colinearidade e igualdade no recálculo. Desenho contínuo a partir do último vértice, até Esc ou fechar uma área; cruzar um trecho anterior ou completar um contorno com linhas já existentes também encerra a cadeia.
- Antigo Tipo renomeado para Traço, disponível em Linha, Retângulo, Círculo e Arco. Normal e Construção nas quatro ferramentas; Infinita apenas em Linha e Retângulo. Retângulos infinitos criam quatro retas auxiliares, sem participar de áreas e extrusões.
- Retângulo Normal por cantos opostos, Do centro para fora com largura/altura totais e Inclinado em três cliques: início/fim do lado e largura perpendicular. Medidas digitadas do lado, ângulo e largura são preservadas.
- Círculo Normal por centro/raio, Dois pontos pelos extremos do diâmetro e Duas linhas perpendiculares com tangência às retas suporte. Após selecionar as linhas, mouse define quadrante/raio; raio também aceita digitação. Tangência e cotas permanecem no histórico e na persistência.
- Variações em botões com ícones vetoriais próprios e seleção visível, como em Traço. Painéis sem descrições; dicas aparecem ao passar o mouse. Nova seção Medidas com campos editáveis sincronizados ao canvas, inclusive antes do primeiro clique. Campos acompanham a etapa atual e respeitam Tab, Enter e Esc; o ícone junto ao cursor também acompanha a variação.
- Formato interno 19 reconhece a tangência entre círculo completo e reta, preservando a leitura dos projetos anteriores.

### Tipos de linha, ângulos relativos e medidas flutuantes

- Painel Linha com os tipos Normal, Construção e Infinita, cada um com ícone e destaque do ativo. Construção e Infinita são tracejadas laranja e não participam de áreas nem extrusões; retas infinitas atravessam o canvas e oferecem captura/interseções além dos apoios de definição, sem extremos fictícios.
- Ângulo em relação à horizontal quando o primeiro ponto está livre ou à direção da linha capturada no início. Cotas relativas acompanham a linha de origem no recálculo. Captura perpendicular ao iniciar, terminar ou cruzar outra linha, respeitando medidas digitadas e suspensão por Alt.
- Prévia angular tracejada preta; comprimento junto ao mouse e ângulo junto à prévia, em campos independentes sem caixa compartilhada. Tab, Enter, Esc, expressões e medidas travadas são preservados; campos próximos recebem folga para não se sobrepor. Ferramentas de desenho exibem seu ícone ao lado do cursor sobre a área de edição.
- Formato de projeto 18 registra retas infinitas e cotas angulares relativas, com leitura dos projetos anteriores. Preservados desfazer/refazer, cópias, espelhamento e padrões de retas auxiliares; retas infinitas não recebem cota de comprimento, aparo ou refinamento de extremos.

### Abas espaçadas e painéis por ferramenta de rascunho

- Abas de ferramentas com títulos mais espaçados e área de clique maior; abas de projetos com mais margem. Um `|` separa abas consecutivas não selecionadas.
- Novo rascunho realça o plano da origem ou a face plana sob o mouse. A consulta exata roda em segundo plano, respeita visibilidade, bloqueio e corte de seção e descarta respostas antigas; faces curvas e malhas não recebem destaque de suporte.
- Entrar no modo rascunho não abre mais a janela genérica Rascunho. Linha, Retângulo, Círculo, Arco, Aparar e Selecionar abrem seus painéis somente quando escolhidos; as novas configurações permanecem a definir. Selecionar mantém diagnóstico, reparo e lista de restrições; operações e restrições conservam seus formulários existentes.

### Repositório público

- Código publicado em <https://github.com/trocolis/3dnator>, sob licença MIT. A CI do GitHub Actions passa a rodar a cada push e pull request.

### Cubo de vista compacto e giros pela face predominante

- Removida a barra permanente acima da área de edição com vistas, enquadramento, seletor de projeção e indicação do contexto. A área 3D passa a ocupar esse espaço.
- Controle com 80% do tamanho da referência anterior, em azuis claros, mantendo chanfros largos e duas setas preenchidas. Nomes das faces voltam ao cubo, incluindo Frente, Superior e Inferior. Home volta ao desenho de contorno e recebe Enquadrar logo abaixo, respeitando modelo ou rascunho ativo.
- Setas e menu giram de 45° em 45° ao redor da normal da face principal predominante. O eixo permanece durante uma sequência para não mudar de plano entre os cliques; escolher outra vista ou arrastar define uma nova referência. Shift+clique ou botão direito na seta inverte o sentido. Arraste livre e seleção das 26 direções continuam disponíveis.
- O menu do cubo reúne projeção ortográfica/perspectiva, enquadramento e os mesmos giros de 45°; os comandos de vista continuam na busca. Os giros conservam alvo, escala e projeção.
- A paleta de rascunho começa abaixo do cubo e recolhida em janelas curtas para manter o navegador acessível, conservando posições ajustadas pelo usuário.

### Projeção dos apoios do modelo no rascunho

- Corrigida a captura no plano do rascunho dos pontos médios das arestas e centros de faces triangulares/retangulares. A projeção passa a usar os mesmos apoios geométricos do movimento ponto a ponto, incluindo centros circulares e quadrantes.
- Removida a amostragem de até 64 vértices por corpo e das primeiras oito ocorrências. As posições do corpo, da ocorrência e do rascunho são compostas antes da projeção ortogonal; apoios coincidentes em alturas distintas são unificados no plano.
- Consulta em segundo plano, com cache atualizado após mudanças do modelo, plano ou estado de seleção/visibilidade. Outros rascunhos visíveis também fornecem seus apoios; o rascunho editado usa sua própria captura local.

### Aparador entre vértices e centros de polígonos

- Cruzamentos e encontros de linhas, círculos e arcos aparecem como vértices de captura no rascunho. São referências derivadas enquanto as curvas permanecem inteiras; aparar materializa os extremos novos.
- Aparar realça o trecho a remover e corta somente entre os encontros/extremos vizinhos. Aceita círculos e arcos, incluindo trechos que atravessam 0°/360° e sentido horário; usa o clique real para evitar que a atração ao centro escolha outro trecho. A edição é atômica, com desfazer/refazer e persistência.
- Centros de triângulos (baricentros) e retângulos (encontro das diagonais) passam a servir de referência no desenho, nos rascunhos auxiliares/de construção e no movimento ponto a ponto de sólidos e malhas planas. Os centros ficam no plano da superfície; centros circulares e pontos médios das arestas continuam disponíveis.
- Encontros e centros são calculados em segundo plano e reaproveitados enquanto a geometria não mudar. Malhas reconstroem faces coplanares para não oferecer centros falsos nas diagonais da tesselação.

### Rascunhos nas quinas chanfradas

- Corrigida a criação de rascunhos nas faces triangulares das quinas de cubos chanfrados e nas faces planas com nomes repetidos. A seleção consulta a superfície exata sob o clique e captura um plano fixo, sem exigir uma referência topológica persistente.
- Funciona também em projetos já salvos, preservando posição e orientação do corpo e da ocorrência. Faces curvas continuam recusadas; operações que mantêm referências paramétricas continuam exigindo nomes estáveis.

### Árvore compacta, rodapé único e navegação da vista

- Removido o cabeçalho “Árvore do projeto” e reduzida a altura das linhas. A raiz usa o ícone do aplicativo e permanece sempre visível; projetos antigos com a raiz oculta são normalizados ao abrir, preservando as opções dos descendentes.
- Rodapé em uma linha: Histórico, desfazer/refazer, linha do tempo e estado/mensagem à direita. Unidade removida; precisão e recuperação ficam no menu Arquivo. Informações e avisos aparecem na barra de status; somente erros abrem diálogos, com fila para preservar falhas recebidas no mesmo quadro. Mensagens longas são abreviadas com texto completo na dica, sem cobrir a linha do tempo.
- Cubo com Home, setas curvas de órbita em passos de 45° e botões de giro da tela em 90°, seguindo o esboço. Home restaura a vista isométrica e enquadra o modelo, preservando a projeção escolhida.

### Importação STEP com triângulos colapsados

- Corrigida a importação de sólidos válidos cuja tesselação OCCT produz triângulos de área zero em costuras/singularidades, como o modelo ESP32 Wroom de 30 pinos. Esses elementos são descartados somente na malha de exibição, preservando o B-rep e a correspondência entre triângulos e faces.
- Detalhes pequenos com área positiva continuam presentes; coordenadas não finitas, falhas numéricas e malhas totalmente degeneradas permanecem erros. Acrescentadas regressões da conversão e ensaio optativo de arquivo externo com salvamento/reabertura e desfazer/refazer.

### Pontos médios, quadrantes e centros de arcos

- Movimentação ponto a ponto oferece a metade do comprimento das arestas, incluindo a metade sobre a curva dos arcos. Malhas oferecem pontos médios nas arestas vivas, sem incluir diagonais internas da triangulação.
- Círculos passam a oferecer quatro pontos de borda, espaçados a cada 90°, além do centro virtual. Arcos também revelam seu centro no plano da curva, inclusive quando ele está fora do material.
- Novos apoios compartilham destaque, captura magnética e seleção na origem e no destino; rascunhos de referência oferecem também os pontos médios das linhas/arcos e os quadrantes dos círculos.

### Captura de centros no movimento ponto a ponto

- Centros virtuais exatos de bordas circulares, inclusive no vazio de furos e arruelas, no plano de cada superfície. Costuras de cilindros deixam de aparecer como vértices artificiais.
- Passar pela região circular revela o centro; aproximar o cursor captura o apoio com uma mira, em alcance constante na tela. Destaque e clique compartilham a mesma escolha na origem e no destino, incluindo vértices e referências de rascunhos.
- Consultas geométricas em segundo plano, com cache por aba/revisão; captura respeita visibilidade, bloqueio, seção e oclusão, sem alterar o documento antes da confirmação.

### Extrusão pelo mouse e seleção automática de operação

- Extrusão aceita áreas fechadas de rascunhos e faces planas de sólidos da peça. Áreas internas, bordas compartilhadas e interseções de linhas/círculos/arcos são reconhecidas sem alterar as entidades do rascunho; furos das faces são preservados.
- Manipulador de distância arrastável no viewport, incluindo inversão ao atravessar o início e preservação do deslocamento inicial. Clicar numa face após escolher o perfil muda automaticamente para Até face; faces inclinadas recortam o prisma inteiro pelo plano exato.
- Operação automática sugere Novo corpo no espaço livre, Unir ao sair de uma face e Remover ao entrar no material. Escolher uma operação manualmente desliga a sugestão; editar uma operação existente conserva seu modo gravado.
- Na remoção, somente o volume realmente subtraído aparece em vermelho translúcido, sobre a prévia do corpo restante. Prévia, arraste e cancelamento não modificam o documento; OK confirma uma única ação com desfazer/refazer.
- Reconhecimento de áreas e cálculos CAD em segundo plano, com cache por revisão. Novas referências de área e extrusão de face são persistidas no formato 17; projetos anteriores continuam legíveis.

### Cotas durante o desenho

- Linha, retângulo, círculo e arco mostram medidas junto ao desenho, atualizadas pelo cursor: comprimento/ângulo em relação ao eixo horizontal, largura/altura, raio e raio/abertura, respectivamente. Digitar trava a medida, Tab/Shift+Tab alterna os campos, Enter confirma a etapa e Esc cancela sem alterar o documento. Apagar um campo libera a medida para acompanhar o mouse.
- Medidas digitadas viram cotas dirigentes com expressões e unidades; os valores não digitados continuam livres. A prévia usa as mesmas medidas da criação, preserva o quadrante do retângulo e evita capturas que contrariem a ponta dimensionada.
- Cotas angulares em radianos no solver, com apresentação em graus, edição, referência, recálculo paramétrico e persistência. Formato de projeto 16; leitura das versões anteriores preservada.

### Faixa Rascunho

- Reorganizada conforme a referência visual do SolidCraft, sem as seções Reference e Configure: Criar (Linha, Retângulo, Círculo, Arco), Modificar (Aparar, Deslocamento, Arredondar), Padrão (Espelhar, Circular, Retangular) e Restringir (Cota e grade de seis restrições). Concluir rascunho fica fixo à direita, acessível também em janelas menores.
- Menus nos nomes dos grupos dão acesso às ferramentas secundárias; cotas, restrições, deslocamentos, cantos e padrões usam painéis com seleção, validação e OK/Cancelar. Alterações passam pelo solver em segundo plano e pelo histórico de comandos, com desfazer/refazer. A paleta flutuante fica compacta, com diagnóstico, reparo e lista de restrições.
- Intenções geométricas ficam no módulo de rascunho, sem dependência da interface, e preservam a geometria e os identificadores quando uma edição falha. Ícones vetoriais próprios distinguem criação em verde e restrições em vermelho.

### Área 3D sem painel Propriedades

- Eliminados o painel lateral Propriedades, seu botão na faixa e sua entrada na busca. A área 3D ocupa todo o espaço à direita da árvore.
- Controles de rascunho passam a uma janela contextual arrastável e recolhível; Medir seleção abre sua própria janela com medições, dimensões e informações da malha. A cor do corpo fica no menu de contexto da árvore, junto aos comandos existentes de renomear, mover, exportar e excluir.

### Menu Mover na aba Peça

- Mover passa a abrir o mesmo menu de opções da Montagem: **Livre**, **Face a face**, **Aresta a aresta** e **Vértice a vértice**, com ícones e descrições.
- Na aba Peça, Livre mantém o modo de mover corpos, e os alinhamentos aceitam apenas corpos da peça ativa como origem. Geometria de outras peças e rascunhos de referência continua disponível como destino.

### Simplificação das abas

- Removidas as abas **Inserir** e **Vista**. A faixa mantém Montagem, Peça, Inspecionar e a aba contextual Rascunho.
- As ferramentas continuam acessíveis pela busca por S e pelos controles de câmera, menus e painéis existentes. O cenário de captura de inserção de outro projeto agora usa Montagem.

### Aba Peça

- Modelo passa a se chamar **Peça**, com os grupos: Novo rascunho; Criar (Extrusão, Revolução, Furo); Acabamento (Arredondar, Chanfro, Casca); Modificar (Mover, Copiar, Combinar, Dividir).
- Mover e Copiar têm entradas e modos próprios, com o mesmo manipulador e a prévia ao vivo. Copiar conserva o original e cria um corpo independente; Mover altera sua posição. O menu de contexto mantém o painel com alternância Mover/Copiar.
- Espelhamento e padrões permanecem disponíveis na busca de ferramentas; parâmetros continuam em Inspecionar.

### Cancelamento de pré-visualizações

- Cancelar uma ação restaura automaticamente a cena do documento, sem esperar outra interação no visualizador. Corrigidos o redesenho ausente ao encerrar a prévia e a chave de uma preparação de cena já concluída que bloqueava o retorno à cena original.
- Regressões verificam Cancelar/Esc na cópia, restauração da malha e preservação do documento/desfazer, além de impedir redesenho contínuo quando o editor fica ocioso.

### Corpos e etapas do histórico

- A pasta **Corpos** lista os resultados atuais da peça sem acumular as etapas anteriores ocultas. Corpos independentes ocultados manualmente continuam acessíveis pelo olho; etapas antigas explicitamente exibidas também permanecem acessíveis.
- Modificações de um corpo preservam seu nome e sua cor. Os nomes de operações como Arredondamento e Chanfro ficam na linha do tempo; editar uma operação existente reutiliza seu resultado.

### Validação dos campos de montagem

- Mover e origem do padrão circular preservam expressões inválidas ao perder foco, destacam os campos e bloqueiam OK/Enter até a correção. Valores fora da faixa de ±10.000.000 mm/graus são recusados sem truncamento silencioso.
- Enter conclui a digitação antes de confirmar; Esc no campo restaura o último valor válido sem fechar o painel. Zerar limpa também os textos pendentes; o manipulador sincroniza os campos com seus novos valores.
- Estado de digitação pertence à abertura do painel, evitando reaproveitar textos de uma operação anterior.

### Árvore do projeto

- Raiz **Projeto** com **Peças**, **Rascunhos** (de referência) e **Origem**; cada peça com **Corpos**, **Sub-peças**, **Rascunhos** e **Origem**, recursivamente. Pastas vazias dentro das peças não aparecem.
- Olho em todos os nós. A visibilidade das pastas é do documento (`ProjectFolders`, `PartFolders`, comando `SetFolderVisibility` com desfazer) e se combina com a de cada elemento, que é preservada; elementos ocultos por um nó acima aparecem esmaecidos. Cena, seleção, folhas e exportações seguem a visibilidade efetiva.
- Origem por peça, oculta por padrão, desenhada em cada ocorrência; a origem do projeto passa a ser gravada.
- Lápis de edição em cada peça: entra na edição (construção), abre o nó e troca a faixa para Modelo; clicado de novo, volta à montagem. Substitui o círculo de ativação.
- Formato `.3dnator` versão 15, com `folders` opcional no projeto e nas peças.

### Refinamento da aba Montagem

- A faixa perde o bloco fixo "Projeto" em todas as abas; ficam só os grupos de ferramentas.
- Peças novas (vazias, STEP, STL ou de outro projeto), novas instâncias e cópias independentes surgem ao lado das ocorrências visíveis, deslocadas em +X com folga de um décimo da maior dimensão (5 a 100 mm), sem sobreposição. As raízes de um STEP vão juntas. `Document::place_beside` e o comando `DuplicateInstance` (Nova instância ao lado da original).
- **Criar a partir de STEP** explica no próprio menu quando o executável foi compilado sem OCCT, e os diálogos de importação aceitam extensões em maiúsculas.
- Painel **Mover** (título só "Mover"): campos de translação e rotação digitáveis, com seleção do valor ao clicar, vírgula decimal, unidades e contas, e destaque de texto inválido. Enter dentro de um campo conclui o valor sem confirmar o painel; antes, o Enter confirmava com o valor anterior e descartava o digitado.
- Os anéis de rotação do manipulador fixam nos múltiplos de 45° a menos de 4°, com o ângulo mostrado junto ao cursor.

### Fluxo próprio da aba Montagem

- **Montagem** passa a ser a primeira aba da faixa e a aberta por padrão, com o fluxo na ordem de trabalho: Rascunho de referência; Nova peça ▾ (criar nova peça, a partir de STEP, a partir de STL); Mover ▾ (movimento livre, face a face, aresta a aresta, ponto a ponto); Copiar ▾ (nova instância, cópia independente, padrão de repetição); Agrupar e Desagrupar. Os menus mostram ícone e explicação de cada subopção.
- **Rascunho de referência**: rascunho da montagem, fora de qualquer peça, desenhado em violeta e listado no topo da árvore. Não pode virar perfil de operação; serve de destino ao mover face a face (plano), aresta a aresta (linhas) e ponto a ponto (pontos). Comandos `AddReferenceSketch` e `RemoveSketch`, com desfazer.
- **Criar nova peça** (`CreatePart`) acrescenta uma peça vazia na raiz, ativa para construção. STEP e STL criam peças novas na raiz, independentemente da peça ativa.
- O painel de alinhamento abre com o tipo escolhido no menu e destaca a origem em azul e o destino em violeta. Ao concluir um rascunho, a faixa volta à aba de origem.
- Formato `.3dnator` versão 14, com `reference_sketches` opcional; projetos anteriores abrem sem rascunhos de referência.

### Edição do histórico e painéis de comando

- Linha do tempo no estilo dos CADs de mercado: reprodução (início, anterior, próxima, fim), marcador arrastável para voltar a uma etapa, reordenação por arraste com verificação de dependências e menu de contexto com editar, editar perfil, renomear, suprimir/reativar, mover marcador e excluir.
- Operações suprimidas preservam parâmetros e referências; dependentes de uma suprimida e operações posteriores ao marcador ficam inativas, com resultados ocultos. Novas operações entram no marcador. Suprimir e recuar não recalculam; reativar e avançar reconstroem o necessário.
- Exclusão com dependentes lista as operações afetadas e oferece excluí-las juntas ou cancelar.
- Falhas no recálculo identificam a operação e suas dependentes. A edição fica pendente, com o último resultado válido exibido, e o usuário escolhe corrigir a operação, suprimi-la junto da edição ou cancelar. Lotes de comandos aplicam a edição e a resolução em uma só ação, com recálculo único no fim.
- Formato `.3dnator` versão 12 com `suppressed` por operação e `history_marker` por peça; projetos anteriores abrem com todo o histórico ativo.
- Painéis de comando ancorados à área 3D substituem a janela de operações: campos de seleção ativos que recebem cliques no modelo (alternando arestas e faces), escolhas por ícone, OK/Cancelar com Enter/Esc e índice topológico em Avançado.
- Perfis escolhidos com um clique nas regiões fechadas do rascunho, com realce sob o cursor. Extrusão em um lado, dois lados ou simétrica, com inversão, deslocamento inicial e extensão até uma face.
- Prévia ao vivo calculada em segundo plano, sem entrar no desfazer, com o resultado em azul. Ao editar, a prévia e as seleções usam o histórico recuado até a operação. O OK publica o cálculo já feito.
- Cenários de captura da interface (`--render-ui-scenario`) para revisão visual de linha do tempo, painéis e falhas.

### Importação de STL e ajustes de edição

- Painel **Inserir malha STL**: depois da leitura, mostra dimensões lidas e resultantes e pede a unidade do arquivo (mm, cm, m, in ou fator) antes de inserir o corpo de malha. Substitui o campo de escala das propriedades.
- Exclusão com dependentes mostra a prévia do resultado atrás do diálogo; Mover/Copiar aceita pivô no ponto escolhido na área 3D; a prévia tinge também componentes com cor de ocorrência.

### Peças de outro projeto

- **Peça de outro projeto** (aba Inserir): escolhe um projeto aberto em outra aba e uma de suas definições, copiada para o projeto ativo como peças novas e independentes, com subpeças, compartilhamento interno e grupos do conjunto. Na construção, a cópia entra como subpeça da peça ativa; na montagem, na raiz. O arquivo de origem não é vinculado nem alterado.

### Precisão de exibição e miniatura

- Casas decimais de exibição (0 a 6) como configuração do documento, escolhidas pela barra de status e aplicadas a medidas, dimensões, coordenadas e parâmetros. Comando `SetSettings` com desfazer; a precisão interna não muda.
- Miniatura isométrica sombreada (`thumbnail.png`) gerada na CPU a cada gravação e exibida na janela de recuperação.
- Formato `.3dnator` versão 13, com `settings` e miniatura opcionais; projetos anteriores abrem com as configurações padrão.

### Reparar rascunho

- Ferramenta **Reparar rascunho** no editor de rascunho: sugere remover entidades duplicadas, remover trechos colineares contidos em outra linha e unir pontas abertas mais próximas que a folga escolhida (por restrição de coincidência resolvida pelo solver).
- Cada correção aparece em laranja sobre o rascunho e só é aplicada se marcada e confirmada; cruzamentos são indicados em vermelho, sem reparo automático. Centros de furo isolados são preservados.

### Corte de seção na área 3D

- Ferramenta **Corte de seção** (Inspecionar e Vista): plano principal (XY, XZ, YZ) ou da face escolhida, deslocamento por campo ou seta arrastável na área 3D e inversão do lado. A parte cortada não é desenhada e o interior exposto vira tampa hachurada, pelo próprio shader.
- O corte é um controle visual da aba, listado em **Análises** na árvore com visibilidade, edição e exclusão; não altera o modelo nem o desfazer. Seleção, pré-seleção, lista de sobrepostos e painéis ignoram a parte cortada.

### Montagem e posicionamento

- Painel **Mover/Copiar** com manipulador 3D (setas por eixo, anéis de rotação e centro livre), campos de translação e rotação no mundo, pivô no centro de massa ou na origem, cópia como nova instância ou corpo independente e prévia ao vivo. Vários elementos se movem em uma só ação de desfazer, inclusive dentro de pais rotacionados.
- Painel **Alinhar** por face, aresta ou vértice, sem restrição permanente; padrões retangular e circular e espelhamento de componentes em painéis com prévia.
- Aba Montagem reorganizada em Componente, Posicionar, Padrões, Organizar e Documentar; Modelo ganha Combinar, Dividir corpo e Mover/Copiar. O antigo diálogo de organização foi substituído.
- Árvore com botão de ativação da peça, nomes de ocorrência, menus de contexto para componentes, corpos e grupos, seção de grupos e renomeação de ocorrências, definições, corpos e grupos.
- Novos comandos `CopyBody`, `RenameInstance` e `RenameGroup`.

### Seleção na área 3D

- Pré-seleção sob o cursor conforme o contexto: face, aresta ou vértice na peça em construção (pelo modo de seleção), componente inteiro na montagem e, com um painel aberto, o tipo pedido pelo campo ativo (arestas, faces ou corpos). Uma etiqueta mostra o nome do elemento.
- Corpos e componentes selecionados são tingidos de azul na GPU, com arestas vivas azuis e profundidade correta; faces e arestas escolhidas e as referências do painel ficam realçadas sobre o modelo.
- Ctrl+clique acrescenta ou retira elementos da seleção, na área 3D e na árvore.
- Seleção por janela: da esquerda para a direita seleciona os elementos inteiramente contidos; da direita para a esquerda, os tocados (tracejado verde). Ctrl acrescenta.
- Segurar o botão esquerdo abre a lista dos elementos sobrepostos sob o cursor, com realce ao percorrê-la.

### Rascunhos no viewport CAD

- Edição integrada ao mesmo viewport 3D, preservando o modelo, a navegação e o cubo de orientação. A câmera se alinha suavemente ao plano; cursor, grade, entidades, cotas e capturas usam o mesmo referencial, incluindo ocorrências rotacionadas na montagem.
- **Novo rascunho** aguarda a escolha de XY/XZ/YZ na origem ou de uma face plana do sólido. Seleção exata no B-rep em segundo plano; faces curvas e malhas são rejeitadas. A origem do rascunho de face é a projeção da origem da peça, independente do ponto clicado.
- Origem da peça visível na área de trabalho, com controle de olho na árvore. Planos de referência aparecem durante a seleção, com alternativas na faixa superior e cancelamento sem modificar o projeto.
- Rascunhos concluídos continuam visíveis. Extrusão, revolução e furo ocultam seu perfil após sucesso; controles de olho na árvore permitem reexibi-lo. Visibilidade participa de desfazer/refazer e do salvamento, e é preservada em recálculos posteriores.
- Cena anterior mantida durante a preparação de uma nova revisão, evitando apagar os sólidos a cada segmento editado. Projetos apenas com rascunhos também são desenhados e enquadrados, incluindo círculos e pontos isolados.

### Navegação e área 3D

- Reconstruído o cubo chanfrado como poliedro fechado, com seis faces, doze arestas chanfradas e oito cantos selecionáveis. A seleção usa os polígonos desenhados, com nomes das vistas que acompanham a orientação da face.
- Removido o indicador XYZ sobreposto. O cubo usa preenchimento por triângulos, contornos sem junções extensas, descarte de faces degeneradas e recorte local para impedir linhas que escapem para o viewport em ângulos rasantes.
- Arraste esquerdo sobre o cubo orbita a câmera sem selecionar ou mover peças. Órbita contínua nos eixos da tela, incluindo passagem pelos polos, compartilhada com Shift + botão do meio.
- Setas horária/anti-horária avançam para a próxima orientação de 90°, alternando 0°/90°/180°/270°; clicar no ângulo restaura 0°. Alvo, zoom e projeção são preservados; a inclinação acompanha a câmera da aba durante a sessão.
- Vistas superior/inferior do cubo e atalhos de câmera passam a usar a mesma orientação, com X para a direita. Raios de seleção, pan, zoom no cursor e matriz da GPU consideram a inclinação da câmera.

### Interface desktop

- Removida a barra de título nativa. Minimizar, maximizar/restaurar e fechar ficam à direita da barra escura do aplicativo; o fundo da barra permite mover a janela e o duplo clique alterna a maximização. Bordas e cantos preservam o redimensionamento, e fechar continua oferecendo a confirmação de alterações não salvas.
- Nova composição inspirada na organização visual do SolidCraft: barra escura de documentos, faixa clara por tarefa, árvore com grupos e controles de visibilidade, painéis recolhíveis e histórico paramétrico com ícones.
- Ícones vetoriais originais para ações de arquivo, modelagem, rascunho, montagem, câmera e navegação, com tooltips, nomes acessíveis e ativação por teclado.
- Acesso direto às operações com a ferramenta já selecionada; sugestões iniciais de perfil e corpo preservam a confirmação da operação. Busca de ferramentas pela tecla S, tolerante a acentos, e aba contextual de rascunho.
- Tema claro compartilhado com o canvas, cores legíveis de restrições/avisos/medições, cubo de orientação identificado por eixos, painéis roláveis e ajuste de largura para janelas menores.
- Rascunhos são enquadrados ao abrir e podem ser reenquadrados pela barra do canvas, incluindo extensões de círculos/arcos. Cotas usam vírgula decimal e omitem zeros desnecessários.
- Capturas separadas de modelo e rascunho, com dimensões opcionais para revisão do layout em 900 × 600 e 1280 × 820. A captura do modelo deixa de abrir um rascunho automaticamente.
- Validação Windows adiada por orientação do usuário e mantida como opção manual na CI; o desenvolvimento atual prioriza Linux e refinamento funcional.

### Modelagem e documento

- Rascunhos 2D com linhas, retângulos, círculos e arcos, geometria de construção, cotas de referência, aparar, estender, offset, espelhamento e refinamento de cantos; solver com indicação de graus de liberdade, conflitos e redundâncias.
- Parâmetros nomeados por peça com expressões e unidades, ligados às cotas dos rascunhos.
- Histórico paramétrico com extrusão, revolução, booleanas, arredondamento, chanfro, casca e transformação, construídos pelo OCCT fora da thread da interface.
- Extrusão com resultado direto (novo corpo, união, corte ou interseção com um corpo da peça) e limite até uma face plana de outro corpo da mesma peça, que acompanha as cotas dessa face.
- Montagem com subpeças, clones independentes e grupos; folhas técnicas associadas com vistas, cotas, corte e exportação PNG/PDF.

### Construção mecânica

- Furos posicionados por centros de círculo ou pontos isolados de um rascunho: cegos com ou sem ponta de broca, passantes, com rebaixo ou escareados, com medidas em expressões e opção de inverter a direção.
- Rosca simplificada registrada no furo (designação, diâmetro nominal e passo), sem hélice na geometria.
- Padrões linear (uma ou duas direções) e circular, espelhamento por plano com opção de manter o original e divisão de corpo por plano.
- Revolução em torno dos eixos do plano do rascunho ou de uma linha do rascunho, que acompanha as cotas.
- Arredondamento e chanfro de várias arestas por operação, com propagação pelas arestas tangentes; chanfro por distância e ângulo; casca com várias faces removidas e paredes para dentro ou para fora, com cantos vivos.
- Padrões linear e circular de entidades no rascunho, com direção dada por uma linha ou centro dado por um círculo ou arco.
- Editor de operações reorganizado por ferramenta, com seções para cada operação.

### Empacotamento e distribuição

- Auditoria do plano com evidências por etapa e critérios de aceite ainda abertos; limites numéricos provisórios de desempenho registrados e benchmark release repetido, identificando corretamente o uso de llvmpipe.
- Empacotadores passam a exigir destinos e arquivos comprimidos novos, preservando dados existentes. Linux rejeita bibliotecas ausentes, usa o diretório explícito do SDK na resolução e copia SONAMEs sem symlinks externos; Windows percorre imports transitivos com `dumpbin`, incluindo dependências além de `TK*.dll`.
- Textos integrais LGPL 2.1/exceção OCCT incluídos em `licenses/` e nos pacotes, com instruções próprias de execução e manifesto de dependências do host. Documentação distingue pacote local verificado de compatibilidade entre sistemas e de revisão completa de licenças para publicação.
- Regressões de preservação de arquivos e relocação do pacote Linux; regressões Windows e ensaios com o pacote movido e PATH limpo preparados na CI. A validação OCCT da CI passa a incluir o desktop e todos os testes do workspace.
- Estratégia de pacotes portáteis autocontidos para Linux (x86_64) e Windows (x64) documentada em `docs/DISTRIBUTION.md`, permitindo executar a aplicação com o núcleo OCCT sem necessidade de instalação global de pacotes do sistema ou privilégios de administrador.
- Script automatizado `scripts/package_linux.sh` que analisa dependências dinâmicas via `ldd`, reúne as bibliotecas compartilhadas do OCCT em `lib/`, gera launcher portátil com `LD_LIBRARY_PATH` e empacota arquivo `.tar.gz`.
- Script PowerShell `scripts/package_windows.ps1` que reúne o executável e as DLLs do OCCT em pasta portátil e gera arquivo compactado `.zip`.
- Inventário detalhado de dependências e avisos de licença em `THIRD_PARTY.md`, incluindo o aviso proeminente e texto integral da *Open CASCADE Technology Exception (version 1.0)* à GNU LGPL 2.1, assegurando conformidade de redistribuição de binários via vinculação dinâmica.

### Exportação para impressão 3D

- Diálogo de exportação STL com escopo (corpo, corpos da peça, cada definição uma vez, montagem em um arquivo ou todas as ocorrências), qualidade rascunho/padrão/alta/personalizada e resumo com arquivos, triângulos, tamanho, dimensões e ocorrências antes de gravar.
- A malha exportada de sólidos vem do B-rep com a tolerância escolhida, não da malha de exibição.
- Nomes por peça e corpo sem colisões; arquivos existentes recebem sufixo em vez de serem sobrescritos.

### Intercâmbio STEP

- Importação XCAF de montagens com nomes UTF-8, subpeças, definições compartilhadas, posições rígidas e cores sólidas por corpo e por ocorrência, sem depender do arquivo original após salvar.
- Exportação STEP de corpo, definição de peça ou montagem visível com cores e hierarquia. Malhas no escopo geram erro explícito, sem omissão ou conversão automática.
- Leitura e escrita STEP serializadas entre threads devido ao estado global do OCCT; ciclos, transformações não rígidas e referências inválidas são rejeitados. Referências externas e peças sem sólidos não fazem parte do intercâmbio suportado.
- Formato `.3dnator` versão 9 acrescenta cor opcional por ocorrência, com migração por valor ausente para versões anteriores. Ensaio `--validate-step` incluído na CI Linux/Windows.

### Montagem

- Padrão linear ou circular de instâncias, que compartilham a definição e ficam reunidas em um grupo; desagrupar libera cada posição.
- Espelhamento de instância como peça nova e independente: sólidos recebem uma operação final de espelhamento no histórico do clone, que continua paramétrico; corpos de malha são refletidos com a orientação das faces corrigida.
- Espelhamento de conjuntos com subpeças aninhadas e rotacionadas, preservando o compartilhamento entre componentes repetidos dentro da cópia independente, os grupos internos e a visibilidade de corpos finais e ocorrências ocultos. Desfazer/refazer inclui a hierarquia inteira em uma ação.
- Clones independentes também preservam os grupos internos do conjunto; grupos externos mantêm seus membros originais.
- Corrigida a conversão de matrizes para ângulos perto de ±90°, evitando erros de orientação ao compor ou espelhar posições.
- Novo rascunho na face selecionada, de qualquer peça visível, com o plano capturado em coordenadas da peça de destino.

### Documentação técnica

- Cotas associativas com amarração a faces e arestas estáveis da geometria e caminhos de ocorrência em montagens. Apoios geométricos extraídos diretamente do B-rep no núcleo OCCT (centroides de faces, centros e pontos radiais em cilindros e esferas, extremidades e centros de arcos em arestas).
- Medições suportadas em folhas: comprimento de arestas (linear ou chamada para curvas), raio e diâmetro em faces e arestas circulares, distância entre faces planas paralelas e ângulo entre faces planas (com arco em vistas ortogonais paralelas ou chamada em vistas oblíquas, e opção de ângulo suplementar).
- Cotas mantêm-se associadas a revisões do modelo: se uma referência geométrica for removida ou oculta, a cota é preservada no documento e identificada com aviso de revisão pendente na folha, sem interromper a geração nem escolher geometria substituta por proximidade.
- Reposicionamento interativo de cotas por arraste de alças na prévia da folha técnica, recalculando traços e textos no papel sem reexecutar o núcleo CAD nem bloquear a interface.
- Formato de projeto `.3dnator` versão 11, persistindo alças e referências associativas nas folhas, com migração compatível para arquivos de versões anteriores.

- Cortes de sólidos calculados pela interseção precisa do B-rep com os planos da folha, incluindo contornos curvos e furos internos. Hachuras são agrupadas por corpo, para que sobreposições de peças distintas não criem vazios artificiais. Traços dos planos e vistas em corte usam as mesmas curvas, amostradas a 0,01 mm.
- Até oito vistas em corte na mesma folha, com identificadores A–A a Z–Z estáveis e layout que se ajusta ao número de cortes; origem e normal do plano editáveis. Orientação retrato/paisagem e resolução PNG de 72 a 300 DPI salvas por folha. Formato de projeto versão 10 com valores padrão para folhas anteriores.
- Vistas de sólidos com remoção exata de linhas ocultas no B-rep pelo OCCT HLR, incluindo oclusão entre corpos e silhuetas curvas. Curvas projetadas são discretizadas com desvio linear de 0,01 mm, independentemente da malha de exibição; folhas de malhas ou contextos mistos continuam identificadas como aproximadas. Cortes de malha mantêm identificação explícita.
- Vista em corte A–A com contornos fechados e hachura a 45°, e traço do plano de corte com as letras de identificação nas vistas ortogonais.
- Legenda com revisão e data editáveis; cotas com vírgula decimal e sem zeros supérfluos.
- Notas de rosca derivadas dos furos do histórico na legenda da folha.
- Ferramenta Medir com volume, área e centro de massa (densidade uniforme), área, raio e diâmetro de faces, comprimento e raio de arestas, ângulo e distância entre faces paralelas e distância mínima entre corpos; exata no B-rep e aproximada, com aviso, em malhas.

### Referências estáveis

- Faces de sólidos recebem nomes pela origem (entidade do rascunho, tampa, arredondamento, face herdada), registrados pelo histórico do OCCT na ponte C++.
- Regiões de extrusão/revolução são identificadas pelas entidades do contorno, e arestas/faces pelos nomes das faces. Mudar uma cota não troca mais a região usada nem quebra arredondamentos, chanfros e cascas posteriores; uma referência que deixa de existir gera erro pedindo nova seleção.
- Clonar uma peça remapeia os donos dos nomes de faces para os novos corpos.
- Formato `.3dnator` versão 7, com nomes de faces por corpo e migração explícita das referências por índice das versões anteriores.
- Seleção de região no editor de operações por lista com área e furos, em vez de índice numérico.
- Seleção gráfica de arestas e faces para arredondamento, chanfro e casca: o clique sobre uma face, ou perto de uma aresta, já gera a referência pelos nomes das faces. A malha dos sólidos guarda a face de origem de cada triângulo, também no arquivo.

### Visualização e desempenho

- Sombras opcionais e desligadas por padrão, controladas por caixa de seleção nas opções de visualização. Iluminação direcional com faces opostas sombreadas e luz suave de preenchimento, combinada com sombra de contato calculada analiticamente no plano de base do modelo, sem alocação de mapas de sombra pesados e compatível com GPUs integradas.
- Cores sólidas sRGB editáveis por corpo, com desfazer/refazer, herança nas operações e preservação na clonagem e no salvamento. Formato `.3dnator` versão 8; projetos anteriores usam a cor neutra. A renderização passa a usar 96 bytes por triângulo, incluindo a cor.
- Projeção em perspectiva opcional, com seletor na área 3D e câmera independente por aba; seleção, medições, movimento livre, zoom no cursor e enquadramento acompanham a projeção. A vista ortográfica continua sendo o padrão. Captura reproduzível por `--render-perspective-preview`.
- Isolamento da seleção, transparência e bloqueio de seleção por instância, e esmaecimento das outras peças durante a construção: controles visuais da aba, sem alterar geometria, histórico ou visibilidade gravada. Corpos translúcidos são desenhados depois dos opacos, com mistura de cores.
- Arestas vivas (dobras acima de 30°, bordas abertas) exibidas por padrão, calculadas uma vez por geometria.
- Triangulação e arestas desenhadas no shader por coordenadas baricêntricas, sem buffer de linhas: a memória da GPU por triângulo caiu de 216 para 84 bytes.
- Vértices de coordenadas idênticas são fundidos na importação STL e na tesselação, sem alterar a geometria. Uma malha fechada passa de 72 para cerca de 12 bytes por triângulo, também no arquivo de projeto.
- Comando `--benchmark` mede abertura, memória, navegação e recálculo, com cada cenário em processo próprio e GPU integrada por padrão. Na malha de 1 milhão de triângulos, o pico de memória caiu de 575 para 312 MiB e a abertura, de 507 para 330 ms.
- Ensaio `--validate-part` do critério de entrega da etapa 2 com o núcleo real.

### Núcleo CAD

- Corrigida a ordem dos componentes OCCT no CMake, que impedia a configuração com qualquer SDK 7.x.
- Ponte compilada e validada com OCCT 7.9.3 no Linux: os nove passos de `--validate` e os testes de nomeação com o núcleo real foram aprovados.
- Mensagens de estatística do OCCT não poluem mais o stdout dos relatórios JSON.
- A tesselação informa a face de origem de cada triângulo, base para seleção gráfica de faces e arestas.

## 0.2.0 — incremento anterior, sem tag

- Documento com peças, múltiplos corpos de malha, instâncias e transformações locais separadas da montagem.
- Comandos atômicos, desfazer/refazer por projeto e identificação de alterações desde o último salvamento; malhas compartilhadas entre snapshots.
- Pacote `.3dnator` versão interna 1, autocontido, com manifesto JSON e malhas f64; gravação temporária validada antes da substituição do arquivo principal.
- Abas independentes, árvore de peças/corpos, importação STL conforme contexto, renomeação, visibilidade, posicionamento numérico e exclusão de corpos.
- Diálogos nativos, Ctrl+S, exportação de corpo STL e confirmação de fechamento com Salvar/Descartar/Cancelar.
- Recuperação automática separada, com opção de restaurar cópias ao abrir o aplicativo e marcador que impede remover projetos normais na limpeza.
- Preparação de cena e buffers CPU em segundo plano, proteção contra resultados de outra aba/revisão e captura da interface real sem janela.
- Ensaio CLI do ciclo importar/salvar/reabrir sem o STL original, recuperação e exportação; CI estendida para esse fluxo.

## 0.1.0 — protótipo técnico anterior

Nenhuma versão foi publicada ou marcada com tag nesta sessão.

### Implementado no protótipo

- Workspace Rust com geometria própria, fronteira de núcleo CAD, intercâmbio, renderizador e aplicação experimental.
- Suporte em L de referência com volume e dimensões conhecidos.
- Leitura STL ASCII/binário com escala e limites de recursos; exportação binária com verificações de geometria e precisão, sem sobrescrita silenciosa.
- Visualizador egui/eframe/wgpu em português, com iluminação simples, câmera ortográfica, deslocamento, órbita, zoom no cursor e vistas predefinidas.
- Captura real do renderizador e nove testes de geometria, STL e navegação.
- Licença MIT para o código próprio, inventário inicial de dependências e documentação de execução e validação.

### Preparado para validação

- Ponte C++ opcional para OCCT 7.x, isolada por API Rust própria, com gestão de propriedade e conversão de exceções.
- Ensaios de sólidos, corte, chanfro, arredondamento, tesselação e ida e volta STEP/STL, com relatório de tempo e memória.
- Workflows de compilação/testes Rust e validação OCCT em Linux e Windows. Execução com SDK OCCT e execução Windows ainda pendentes.

### Planejado para o editor

- Projeto autocontido `.3dnator`, recuperação automática, abas, peças, corpos, instâncias, clones, grupos e montagem.
- Rascunhos com restrições, cotas, referências, diagnóstico, reparo, aparar, estender, offset e geometria de construção.
- Histórico paramétrico, parâmetros/expressões, operações atômicas e desfazer/refazer.
- Extrusão, revolução, booleanas, refinamentos, casca, furos, roscas simplificadas, padrões e espelhamento.
- Importação STEP como peças e STL como corpos de malha, referências independentes entre peças e exportação STL por corpo.
- Medição, cotagem, cortes, folhas técnicas associadas ao modelo e exportação PNG/PDF com opção blueprint.
- Linux e Windows como plataformas iniciais; referência de desempenho com 8 GB de RAM e GPU integrada.

Escopo e decisões completos em README.md, ARCHITECTURE.md e ROADMAP.md. Recursos planejados não são funcionalidades entregues neste protótipo.
