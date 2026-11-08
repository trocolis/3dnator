# Histórico de mudanças

## Não lançado

## 0.52.0 — 08/11/2026

- Extrusão: as faces planas aptas se destacam sob o cursor também em cenas grandes (STEP importados, antes sem pré-seleção acima de 400 mil triângulos) e quando a divisão da face em células falha (a face inteira aparece em vez de nada).
- Ferramentas que esperam face (Extrusão, Casca, Remover faces, Dividir etc.): **↑/↓** percorrem as faces sob o cursor, da frente para trás, inclusive as vistas por trás, como a face oposta de um cubo; o rótulo mostra a posição e o clique escolhe a face realçada. A escolha de plano do rascunho passa a oferecer também essas faces. Nova consulta `Document::pick_layers_clipped_with`, com todos os acertos do raio em cada corpo.
- Clicar numa face ou aresta da peça em construção realça só a face ou aresta, não o corpo inteiro (que continua selecionado para os comandos).
- Ponto a ponto: pontos atrás do material, cujas marcas já apareciam, agora podem ser capturados (o corte de seção continua excluindo os removidos); empatados na tela, ganha o da frente.
- Ponto a ponto: a seta dos centros virtuais vem de fora do material, pela normal externa da face plana que contém o círculo (informada pela ponte OCCT); um centro do outro lado do corpo recebe a seta de lá.

## 0.51.0 — 08/11/2026

- O tamanho da faixa (Maior, Normal, Compacto) passa a ser escolhido só em Configurações → Aparência → **Tamanho da faixa**; o botão ao lado da busca saiu.
- Ponto a ponto: a seta aparece só nos centros virtuais (no vazio, como o de um furo ou o de um arredondamento côncavo); pontos médios e centros sobre o material ficam só com a marca. A ponte OCCT informa se o centro cai dentro de uma face plana vizinha, e a pré-seleção diz "Centro circular" ou "Centro circular (virtual)".
- Espelhar (montagem): campo **Linha de referência**. Um clique numa linha de rascunho de referência faz o plano escolhido passar por ela; a linha precisa ser paralela ao plano (sempre vale com o rascunho paralelo a ele), senão o painel avisa e não monta o comando.
- Escolha de plano com planos sobrepostos: **↑/↓** percorrem os planos sob o cursor (os da origem e de trabalho, depois as faces planas do modelo, da frente para trás, inclusive atrás de outros objetos) e o clique escolhe o destacado. O rótulo mostra a posição, como "Corpo (2/3, ↑/↓)".
- A linha do tempo abre na última alteração e a acompanha; um marcador movido para o meio do histórico é trazido à vista.

## 0.50.0 — 08/11/2026

- Menus de contexto: cada item ganha fundo destacado sob o mouse, na largura toda do menu (também na busca de ferramentas e na lista de projetos recentes). Antes, os itens com ícone não reagiam ao mouse.
- Árvore do projeto: a mesma largura recolhida e expandida, ajustável arrastando a borda direita (200 a 640 pontos, até 60% da área de edição) e guardada nas preferências.
- Ponto a ponto: passar o mouse sobre um centro, ou escolhê-lo como origem ou destino, realça também o perímetro do círculo ou arco. Centros aparecem como losango verde com ponto (como no rascunho) e pontos médios de aresta como triângulo âmbar, com setas maiores na cor da marca, perpendiculares ao plano do ponto: o eixo do círculo nos centros e a bissetriz das faces nos pontos médios (a ponte OCCT passa a informar essa normal), sempre vindas do lado do observador.
- Faixa de ferramentas em três tamanhos, escolhidos no botão ao lado da busca e guardados nas preferências: **Maior**, com o nome de cada seção sob os botões; **Normal**, como antes; **Compacto**, uma linha só de ícones.
- A linha do tempo do histórico aparece só nas abas Construção e Rascunho; na Montagem e na Impressão, o rodapé mantém desfazer/refazer e o estado.
- Cenários de captura `ribbon-large` e `ribbon-compact`.

## 0.49.0 — 08/11/2026

- Árvore: um nome abreviado aparece inteiro enquanto a linha está sob o mouse ou selecionada, sobre fundo opaco e mesmo além da borda da árvore.
- Medir: na decomposição ΔX, ΔY e ΔZ, cada cota fica no meio do seu trecho e do lado de fora do triângulo, com borda e linha de chamada na cor do eixo (a distância total, em verde), afastada até não cobrir outra cota nem as linhas medidas. Antes, as cotas ficavam deslocadas para baixo, longe dos trechos e misturadas entre si.
- Ponto a ponto: entre as marcas do corpo mapeado, centros de círculos e arcos (no material ou virtuais) ganham uma seta diagonal, e pontos médios de aresta, uma seta vinda de cima.
- Cenário de captura `align-points` (`--render-ui-scenario`): ponto a ponto com o corpo já mapeado.

## 0.48.0 — 08/11/2026

- **Grupo rígido:** o menu do grupo na árvore ganha **Tornar grupo rígido** / **Tornar grupo organizacional**. Num grupo rígido, mover qualquer membro (Mover, alinhamentos, ponto a ponto, arraste) move os demais pelo mesmo deslocamento, numa só ação de desfazer, e a prévia já mostra o grupo inteiro. O grupo ganha a marca **Rígido** na árvore e os painéis de mover e alinhar avisam quantos membros vão junto. Os membros continuam com posições próprias, sem origem nova. Clicar num membro seleciona só ele; copiar, padrão e espelhar atuam só sobre a escolha. Um componente pertence a no máximo um grupo rígido. Mover um grupo organizacional também leva os parceiros rígidos dos membros, cada um uma vez. Clonar ou importar um conjunto preserva a rigidez dos grupos internos. Formato 53: `groups[].rigid`.

## 0.47.0 — 08/11/2026

- Nova ferramenta **Remover faces** (faixa Sólido, ao lado de Arredondar e Chanfrar), no estilo da exclusão de faces do Fusion: escolha as faces de arredondamentos, chanfros, furos ou ressaltos e as faces vizinhas se estendem até fecharem o sólido. Serve sobretudo para STEP importado, em que o arredondamento não está no histórico. A operação usa o `BRepAlgoAPI_Defeaturing` do OCCT, entra no histórico como as demais (editável e recalculada ao mudar parâmetros) e as faces estendidas conservam o nome, então operações posteriores continuam achando a mesma face. Se alguma face não puder ser removida (por exemplo, a tampa de um prisma), a operação falha inteira, sem sólido removido só em parte. Formato 52: operação `RemoveFaces`.
- Isolamento visível: o componente isolado ganha a marca **Isolado** na árvore, e um balão no topo da área de edição mostra o nome dele (ou a quantidade de componentes isolados) com o botão **Sair do isolamento**. Antes, nada indicava que a cena estava reduzida.
- Grupos ganham o olho de visibilidade na árvore: um clique mostra ou oculta todos os membros numa só ação de desfazer. O grupo continua sem visibilidade própria (aparece visível enquanto algum membro estiver), então o formato não muda.
- O painel de **Unir áreas** não mostra mais os campos de espessura e direção da casca, que não tinham efeito.

## 0.46.0 — 08/11/2026

- Janela de recuperação: cada cópia ganha **Excluir**, ao lado de **Recuperar**, com confirmação em **Excluir definitivamente**. Só arquivos marcados como cópia de recuperação são apagados; sem cópias restantes, a janela fecha.
- Aba Montagem: **Espelhar** ao lado de **Copiar**, abrindo o painel de espelhar componente (antes só no menu da árvore).
- Menu de contexto de cópias compartilhadas (peças e referências) ganha **Tornar independente**: a instância passa a usar uma definição própria, no mesmo lugar e com o nome dela, numa só ação de desfazer. O item só aparece quando outra instância usa a mesma definição.

## 0.45.2 — 08/11/2026

- Alinhamentos (ponto a ponto, face a face e aresta a aresta) movem a referência da peça em edição: o painel descartava o movimento sempre que a origem não era um corpo da peça ativa, mesmo com a seleção da referência já aceita.

## 0.45.1 — 08/11/2026

- **Renomear** uma peça na árvore leva junto o nome da definição (exibido na edição e usado nas cópias) quando só essa instância a usa ou quando ela ainda tem o nome automático, numa só ação de desfazer. Cópias compartilhadas (nova instância, copiar componente) herdam o nome da instância copiada em vez do nome da definição.
- Na construção, as referências da própria peça (pasta Referências dela) podem ser selecionadas e movidas (Mover, alinhamentos e ponto a ponto), sempre inteiras, em vez do aviso "O corpo selecionado pertence a outra peça". Um clique dentro de um STEP de referência escolhe a referência de nível mais alto. Referências do projeto ou de outras peças continuam fora, com aviso próprio que indica a Montagem ou o arrastar para as Referências da peça.
- **Converter para objeto de referência** não falha mais em silêncio: o item fica clicável e, com cálculo em andamento, rascunho aberto, falha pendente ou ferramenta não confirmada, o aviso diz o que falta. Ao entrar na edição de uma peça grande, o cálculo inicial bloqueava a conversão sem explicação.

## 0.45.0 — 08/11/2026

- Cada peça tem a própria pasta **Referências** na árvore, separada de Sub-peças, para os modelos de referência posicionados dentro dela. Referências (e peças ou subpeças a converter) podem ser arrastadas entre as pastas Referências do projeto e das peças, ou soltas sobre o nome de uma peça, mudando de dono na mesma posição no mundo e numa só ação de desfazer. Formato 51: `folders.references` grava a visibilidade da pasta de cada peça; projetos anteriores abrem com ela visível.

## 0.44.0 — 07/11/2026

- Ponto a ponto mais leve, na montagem e na construção: cada campo começa pela escolha de um corpo (realçado ao passar o mouse; em cenas grandes, só pelo clique) e somente os apoios desse corpo são calculados, mostrados como marcas discretas e capturados. Raios e oclusão ficam restritos à ocorrência dele, com conferência pelas demais peças só para até três candidatos e em cenas abaixo de 400 mil triângulos. O painel mostra o corpo mapeado e **Trocar corpo**; clicar noutro corpo troca o mapeamento. Antes, cada movimento do cursor varria os apoios e os triângulos de toda a montagem, o que travava a interface em STEPs grandes. Pontos de rascunhos e origens continuam valendo como destino sem escolher corpo.
- Menu de contexto das referências na árvore ganha **Exportar STEP…**, com os corpos e subpeças da referência, como já havia no menu das peças.
- Peças e subpeças podem ser arrastadas até a pasta **Referências**: a subpeça sai do pai, vai à raiz na mesma posição no mundo (pela ocorrência arrastada) e vira referência, numa só ação de desfazer. A pasta mostra a dica ao passar o mouse; a conversão segue as mesmas restrições de **Converter para objeto de referência**.

## 0.43.0 — 07/11/2026

- Movimento ponto a ponto, na montagem e na construção: todos os elementos exibidos servem de destino. Além dos corpos e dos rascunhos de referência, valem os rascunhos das peças (na pose de cada ocorrência, com extremos, pontos médios, centros e quadrantes) e as origens visíveis do projeto e das peças; pontos atrás de material ou removidos pelo corte não são oferecidos. Um ponto do modelo capturado mais perto do cursor deixa de perder para um apoio de rascunho a até 12 pt. O realce dos apoios continua em cenas acima de 400 mil triângulos (como STEPs grandes), com o intervalo mínimo da pré-seleção.

- Importação STEP não falha mais com "Uma face não possui triangulação." quando o BRepMesh rejeita faces válidas (cones e B-splines exportados pelo Fusion). A ponte tenta, face a face, remalhar com ângulo mais folgado, uma cópia NURBS com a qualidade pedida e, por fim, deflexões maiores; só então informa o erro. Faces de área nula (lascas entre duas arestas coincidentes, como um toro de largura zero num suporte do Fusion) não exigem mais triângulos: ficam fora da malha de exibição, sem alterar o B-rep.

- Analisadas as quatro pendências atuais de `AJUSTES.md` em `docs/PLANO_AJUSTES_VIEWPORT.md`, com viabilidade, evidências no código, ordem de implementação e critérios de aceite para precisão, cubo e corte de seção. Sem alteração funcional.

- Documentado em `docs/PLANO_AJUSTES.md` o diagnóstico de viabilidade dos 11 itens pendentes de `AJUSTES.md`, com sequência de implementação, limites de compatibilidade e critérios de aceite. Nenhuma funcionalidade foi alterada nesta análise.

## 0.42.0 — 07/11/2026

- Configurações reunidas num painel único, com as seções Aparência e Desenho. Rótulos à esquerda e entradas alinhadas numa coluna de largura comum; tema, idioma e unidade passam a seletores (combobox) e as descrições aparecem em balões ao passar o mouse.
- Atalhos ganham janela própria, aberta por **Configurar** na seção Desenho: seções por contexto em três colunas (ícone, ação e tecla). **Sobre** também abre janela própria. A ajuda sai das configurações por enquanto, e com ela o botão de recuperação, que continua aparecendo na abertura quando há cópias automáticas.
- Cenários de captura `settings-drawing` e `settings-help` removidos; `settings-shortcuts` e `settings-about` abrem as novas janelas.

## 0.41.1 — 07/11/2026

- Árvore flutuante ocupa toda a altura disponível até a barra inferior; rolagem aparece somente quando os nós excedem esse espaço, sem o limite inicial de altura das áreas do egui.

## 0.41.0 — 07/11/2026

- Árvore do projeto sempre flutua sobre a área de edição, aberta ou recolhida, sem painel atrás dos nós nem coluna lateral reservada. Cabeçalho, rolagem, menus e balões de edição continuam acessíveis.

## 0.40.2 — 07/11/2026

- Ícones exibem uma única dica: nome completo e atalho na faixa, explicação nas opções e informações da árvore, preservando nomes acessíveis e ativação por teclado.

## 0.40.1 — 05/11/2026

- Mitigação para a interface Flatpak paralisada após inatividade: o lançador prefere X11/XWayland quando disponível, preservando Wayland em sessões sem X11. `THREEDNATOR_WAYLAND=1` permite testar o caminho nativo. A ocorrência capturada estava na espera de eventos Wayland, sem trabalhador CAD ativo; a causa interna e a estabilidade prolongada ainda precisam de confirmação.

## 0.40.0 — 05/11/2026

- Extrusão seleciona conjuntamente áreas coplanares de faces de corpos diferentes e de rascunhos distintos da peça em edição, com uma operação e uma ação de desfazer. A seleção alterna por clique e conserva todos os realces; o recálculo verifica o plano e usa identidades persistentes por perfil.
- Formato 50 registra a extrusão de perfis heterogêneos, mantendo leitura dos anteriores. Clonagem remapeia fontes e conserva identidades; perder uma referência ou a coplanaridade falha atomicamente.

## 0.39.0 — 05/11/2026

- Medir reconhece pontos, linhas, círculos e arcos de rascunhos de peça e de referência, incluindo ocorrências posicionadas. Hover e clique compartilham a consulta; comprimento e raio são analíticos, e os alvos participam das distâncias e ângulos existentes. Ocultação, isolamento, bloqueio, corte e oclusão por material são respeitados.

## 0.38.8 — 05/11/2026

- Ferramentas secundárias das seções passam a botões visíveis; a faixa quebra os grupos em novas linhas para mantê-los dentro da janela. Imagem, Estender, Espelhar, Excluir, Parâmetros e Corte de seção não dependem mais de menus de grupo. Restrições, Varredura e padrões de corpos também ficam expostos.

## 0.38.7 — 05/11/2026

- O realce do Medir permanece durante o movimento contínuo do cursor enquanto a próxima consulta termina. Mudanças na câmera, seção, aba, revisão ou visibilidade invalidam o resultado antigo; o trabalhador continua atualizando o candidato sem quadros vazios.

## 0.38.6 — 05/11/2026

- Medir, Arrastar e Rotacionar usam somente o cursor do sistema, sem ícone adicional. A suspensão de uma ferramenta de construção também esconde seu distintivo até a navegação terminar.

## 0.38.5 — 05/11/2026

- Medir usa cursor em mira sobre o canvas. Arrastar/Rotacionar e o cubo conservam seus cursores; sair do Medir ou apontar para os painéis restaura o comportamento normal.

## 0.38.4 — 05/11/2026

- Distintivo junto ao cursor compartilhado pelas ferramentas de Construção, montagem, Medir, navegação e rascunho, incluindo Texto e modificações. Selecionar conserva o cursor sem distintivo. A navegação suspensa mostra seu próprio ícone e retoma o da ferramenta ao voltar; painéis e cubo não recebem o distintivo do canvas.

## 0.38.3 — 05/11/2026

- A célula escolhida como perfil da Extrusão conserva seu realce ao passar o mouse sobre outra área, inclusive com distância zero ou entrada inválida. O cálculo do realce tem trabalhador e cache próprios, independentes da prévia do sólido; troca de perfil, revisão, posição ou cancelamento descarta respostas antigas.

## 0.38.2 — 05/11/2026

- Medir realça apenas o vértice, a aresta ou a face candidata ao clique, inclusive na montagem. A classificação usa o mesmo alcance e consulta exata do clique, em trabalhador; respostas de outra posição, seção, aba ou revisão são descartadas.

## 0.38.1 — 04/11/2026

- Pré-seleção na seção passa pelo recorte e teste de profundidade do viewport: contornos não atravessam material opaco nem mostram a parte removida. Rótulos e mira permanecem na interface; seleção persistente e manipuladores conservam seus contratos.
- Câmera e plano invalidam o cache de hover. Clique, sobrepostos, captura de pontos e suporte de rascunho consultam superfícies conservadas pelo corte; tampas bloqueiam alvos atrás do material sem criar faces CAD artificiais.

## 0.38.0 — 04/11/2026

- Corte de seção prepara tampas no plano e contornos por corpo/ocorrência em trabalhador, preservando cavidades e a divisória entre corpos encostados. Faces traseiras deixam de simular a tampa e não disputam profundidade nas interfaces internas.
- Corrigida a identidade da pasta Imagens em ocorrências repetidas, detectada no cenário de seção com duas instâncias.
- Mudanças de plano descartam tampas antigas; malhas com contorno aberto não recebem fechamentos inventados. A seção continua sendo apenas apresentação, sem mudar B-rep ou formato do projeto.

## 0.37.2 — 04/11/2026

- Clicar numa face principal do cubo alinha a vista em quartos de volta e conserva o alto predominante da orientação anterior, inclusive nos polos. Home e os cliques em cantos/arestas mantêm suas vistas canônicas.

## 0.37.1 — 04/11/2026

- Precisão linear em campo numérico único, com padrão 0,1 mm. Trocar a unidade conserva o passo físico; projetos existentes preservam a precisão gravada e pontos anteriores não são alterados.

## 0.37.0 — 04/11/2026

- **Texto** mantém o painel aberto após os dois cliques de posicionamento e orientação. Só a confirmação publica o bloco, em uma única alteração de desfazer; cancelar descarta a candidata.
- A orientação captura vértices, centros, pontos médios, curvas e referências da cena, com o mesmo campo angular flutuante das ferramentas de desenho. Um ângulo digitado permanece travado durante o movimento do mouse.
- O painel oferece posição X/Y e prévia ao vivo de conteúdo, fonte, estilo, tamanho e rotação. Fontes e solver rodam no trabalhador; transformações reutilizam os contornos unitários. Edições simples preservam IDs, restrições e referências de regiões.
- Resultados de outra revisão, aba ou painel cancelado não podem publicar um texto antigo.

## 0.36.0 — 04/11/2026

- **Dimensionar** aceita a origem local como referência fixa para pontos, centros de círculos/arcos e linhas. A posição do texto escolhe distância alinhada, horizontal ou vertical; o valor aceita expressões e pode ser editado como nas outras cotas.
- Formato 49 persiste as cotas na origem e a ordem dos alvos sem criar pontos auxiliares no desenho. A origem não se move nem entra na coleta de furos.

## 0.35.1 — 04/11/2026

- Furo sugere a direção para o interior do corpo ao escolher o apoio e a base; mantém a direção manual enquanto esses alvos não mudam. Cortes sem interseção são recusados, em vez de criar uma operação sem efeito.
- Centros coletados automaticamente para furos e fixações excluem referências projetadas, linhas da face e apoios de texto. O diâmetro continua sendo o valor explícito do painel.

## 0.35.0 — 04/11/2026

- **Tubo** é o nome da ferramenta em português e espanhol; **Pipe** permanece em inglês. As dicas usam Tubo por trajetória / Tubo por trayectoria.
- Tubo agora aceita círculos e cadeias fechadas de linhas e arcos tangentes, com seção maciça ou oca e sem tampas na emenda. A tangência também é verificada entre o último e o primeiro trecho; quinas, ramificações e autointerseções continuam recusadas atomicamente.
- Formato `.3dnator` 48 registra a semântica de trajetórias periódicas; leitura das versões anteriores permanece disponível.

## 0.34.0 — 04/11/2026

- **Pipe**, em Construção → Criação e na busca: seção circular maciça ou oca, com diâmetro externo e espessura por expressão, ao longo de uma cadeia aberta de linhas e arcos tangentes de rascunho. A escolha de um trecho captura a cadeia; a prévia roda no trabalhador e a confirmação cria um corpo com histórico editável, recálculo e desfazer/refazer.
- Referências da trajetória são remapeadas ao clonar a peça e preservadas no formato `.3dnator` 47. Espessuras inválidas, trajetórias desconectadas, fechadas ou com quinas sem arco são recusadas sem alterar o documento.

## 0.33.0 — 04/11/2026

- **Converter para objeto de referência** no menu da peça, com geometria, histórico e ocorrências preservados. A conversão vale para todas as instâncias da definição e fica indisponível enquanto houver rascunho ou ferramenta aberta. **Criar peça** reverte a marca.
- Referências aninhadas agora usam a mesma apresentação protegida das referências da raiz na árvore. Converter a peça ativa encerra sua edição e volta à Montagem.

## 0.32.2 — 04/11/2026

- Selecionar na barra de navegação retoma a ferramenta aberta após Arrastar/Rotacionar, conservando painéis, valores, alvos e prévias. Esc e a troca de ferramenta mantêm o cancelamento explícito.
- Corrigida a detecção de ferramenta ativa para incluir Texto do rascunho.

## 0.32.1 — 04/11/2026

- Corrigido o contraste dos nomes das faces do cubo de vista no tema escuro.
- Removidos os acessos de criação de Rótulo; operações existentes continuam editáveis pelo histórico e fontes continuam disponíveis para Texto.
- Na Construção, Furo passou para Criação e Casca para Modificação.

## 0.32.0 — 02/11/2026

- **Linha tangente com Shift:** uma linha que começa num círculo ou arco, com Shift pressionado, desliza a origem sobre a curva e fica tangente a ela, passando pelo cursor. Dos dois pontos de tangência vale o do lado em que a linha começou. Ao confirmar, a origem fica presa à curva e a linha recebe a restrição de tangência, que acompanha mudanças de raio. Soltar o Shift volta ao traço comum a partir do último ponto.
## 0.31.2 — 02/11/2026

- **Diagnóstico**, ao lado de Dimensionar, virou um botão do tamanho das demais ferramentas da aba Rascunho, com legenda. Para a faixa continuar cabendo em 1280 pontos, **Espelhar** passou para o menu ▾ da seção Modificar.

## 0.31.1 — 02/11/2026

- Corrigido: o Texto do rascunho enchia o desenho de alertas de pontas não ligadas e o rascunho não concluía. O núcleo entrega um trecho por aresta da fonte, e cada emenda dentro de uma letra virava um par de pontas soltas sobrepostas. Agora os trechos se juntam em contornos fechados ao criar ou editar o texto. O diagnóstico também deixou de sugerir ligar ou aproximar pontas entre duas referências fixas (texto, projeção, linhas da face), o que valia para textos gravados antes desta versão e deixava o rascunho redundante.
- Corrigido: recolocar um texto com um trecho menor que um nanômetro recriava as linhas com IDs novos, o que podia perder as áreas escolhidas.

## 0.31.0 — 02/11/2026

- Balões de edição numa fila no canto, junto da árvore (ou da barra dela, recolhida): o da peça em edição, com Salvar; o do rascunho em edição, com o nome e o botão **Concluir**; e o dos graus de liberdade. Todos têm o estilo da árvore recolhida, a mesma altura e não quebram linha; nomes longos ficam abreviados, com o nome inteiro na dica.

## 0.30.1 — 02/11/2026

- Corrigido: no tema escuro, o texto das cotas do rascunho ficava ilegível, claro sobre um fundo fixo cinza-claro. O fundo da etiqueta e o véu das etapas posteriores ao marcador do histórico agora seguem o tema.

## 0.30.0 — 02/11/2026

- **Importar**, na Montagem, deixou de ser um menu: o botão abre direto o seletor de arquivos com STEP, STL e 3MF, e o modelo entra como referência. A ferramenta não cria mais peças; para isso, use **Criar peça** no menu da referência, na árvore. Saíram os itens Referência STEP/STL/3MF e Criar a partir de STEP/STL.

## 0.29.1 — 02/11/2026

- Linha contínua: o traço encerra quando a reta é infinita ou quando a ponta termina sobre outro elemento (vértice, linha, curva, centro, interseção, tangente ou projeção). A sequência só continua numa ponta solta, alinhada ou num passo da precisão.
- `scripts/package_flatpak.sh` aceita `FLATPAK_RUNTIME` (padrão 25.08) e usa o Platform como Sdk quando o Sdk não está instalado. Um executável compilado numa distribuição recente, com glibc mais nova, roda no runtime 26.08 para instalação local.

## 0.29.0 — 02/11/2026

- **Medir** foi refeito, sem painel. Cada clique escolhe um vértice, uma aresta ou uma face, e a medida aparece desenhada na cena. Um alvo mostra a própria medida: coordenadas, comprimento (com raio e diâmetro se for circular) ou área. Dois alvos mostram a menor distância, ou o ângulo se forem arestas ou faces não paralelas. Duplo clique no segundo alvo decompõe a distância em ΔX, ΔY e ΔZ. Esc apaga a medição e, sem medição, sai do Medir. Saíram o relatório, o alinhamento e a rotação que ficavam no painel antigo.
- **Junção automática de áreas:** na união, as faces novas da extrusão que continuam faces existentes no mesmo plano viram uma só área. As divisões feitas de propósito pela extrusão zero continuam. A opção **Juntar áreas coplanares** fica no painel; projetos anteriores recalculam sem a junção.
- **Importar como referência:** o menu Importar da Montagem traz STEP, STL e 3MF para a pasta **Referências**. A referência se move, oculta e mede, mas não se edita. **Criar peça**, no menu dela, a transforma em peça. Também passou a ler 3MF (só a malha, em milímetros).
- **Texto** no rascunho, depois de Projetar: texto de uma linha com as opções do Rótulo, sem cor, colocado com dois cliques (ponto de inserção e direção). As letras são linhas fixas que delimitam áreas; o ponto de inserção aceita restrições e cotas. Duplo clique numa letra reabre o bloco. Na faixa, Imagem passou para o menu ▾ da seção, para a faixa caber em 1280 pontos.
- **Configurações:** a aba Geral saiu e a unidade e as precisões foram para Desenho. As unidades agora são milímetro e polegada; projetos em centímetro ou metro abrem em milímetro. Em polegada, a precisão do mouse oferece 1/64, 1/32 e 1/16 in.
- **Aparência:** tema claro, escuro ou do sistema e idioma Português, Español ou English (ou o do sistema, com português como padrão). A ajuda ainda está só em português.
- **Atalhos** configuráveis por contexto, com aviso de conflito. Novos padrões: E Extrusão, R Revolução, F Arredondar e C Chanfrar na Construção, e M Mover na Montagem.
- **Ajuda:** tutorial curto por tarefa e lista dos atalhos em vigor, com busca.
- Peças e rascunhos novos com o nome padrão recebem número em qualquer dos três idiomas ("Part 2", "Boceto 3").
- Formato `.3dnator` 46 também ganha `texts` nos rascunhos, `merge_coplanar` no `Combine`, `reference` nas peças e a unidade polegada.

## 0.28.0 — 02/11/2026

- Rascunho criado sobre a face de um corpo já traz, em amarelo, todas as linhas das faces do corpo nesse plano: contorno, furos e divisões. São referências fixas, sem graus de liberdade, usadas por restrições e cotas e que delimitam áreas, mas não se movem, não se aparam, não se apagam nem se arredondam. Elas não acompanham mudanças posteriores do corpo. Para rascunhos antigos, **Trazer linhas da face** no menu do rascunho na árvore. A projeção lilás ganhou a mesma proteção.
- Vértices próximos se fundem pela precisão de desenho, contra micro-fendas na impressão. Uma ponta nova a menos de um passo de um vértice mais antigo vai para ele e fica ligada, também com valor digitado; perto de uma linha da face ou projeção, vai para a linha. Traços menores que a precisão são recusados. Cada ponto guarda a precisão em que foi criado. Áreas e extrusões fundem as pontas a essa distância, 0,001 mm nos pontos antigos, sem mexer no ponto mais antigo nem no rascunho gravado.
- Corrigido: Projetar e a captura de arestas e vértices de outros corpos pareciam não funcionar em projetos com componentes importados grandes. A ponte com o OCCT remontava os mapas de arestas, faces e vizinhanças do sólido a cada consulta, o que tornava a leitura quadrática. Agora eles são montados uma vez por sólido. No projeto de teste com placa de circuito e ESP32 (123 ocorrências), a consulta do Projetar caiu de 24 s para 2,3 s e a da captura de 33 s para 4,4 s.
- Corrigido: em rascunho sobre uma face, Projetar a própria face ou suas arestas fechava a ferramenta sem acrescentar nada, porque essas linhas já vêm em amarelo. Agora avisa que não há nada novo, mantém a ferramenta aberta e não grava etapa vazia no desfazer.
- Corrigido: editar um rascunho de uma peça com pintura em faces de nome repetido falhava com "A geometria mudou", porque a pintura por índice não acompanhava o corpo refeito.
- Formato `.3dnator` 46: `face` em pontos e entidades de rascunho e `precision` nos pontos.

## 0.27.0 — 02/11/2026

- **Diagnóstico do rascunho** saiu do menu **Restringir ▾** e virou um botão compacto ao lado de **Dimensionar**.
- Ao editar um rascunho, um balão no canto, junto da árvore, mostra os graus de liberdade, ou que ele está totalmente restrito ou em conflito. O valor é calculado já ao abrir, não só depois da primeira edição.
- Na extrusão, o campo da distância abre com foco assim que a seta aparece: basta digitar e dar Enter; Esc devolve o valor anterior. Arrastar a seta continua funcionando.
- Um rascunho novo e vazio sobre uma face enquadra a face, em vez da origem do plano.
- Na Montagem, **Nova peça** cria direto uma peça vazia. Peças a partir de STEP e STL ficam no menu **Importar**, ao lado.

## 0.26.2 — 02/11/2026

- Limpeza do código: saíram os padrões linear e circular do rascunho, sem entrada na interface; a operação `Transform`, que nenhuma tela criava; os comandos `PaintSurface` e `ClearSurfacePaint`, substituídos por `PaintSurfaces`; 45 textos sem uso, os ícones `LockClosed` e `Project` e todos os `allow(dead_code)` que os escondiam. Um projeto que ainda contenha `Transform` é recusado ao abrir. A Transição guiada continua, porque projetos existentes a usam.

## 0.26.1 — 02/11/2026

- Corrigido "Triângulo degenerado ou fora do intervalo numérico" em Verificar sólido, Verificar impressão e na exportação. Bastava uma lasca de milionésimos de milímetro, deixada por um rascunho quase alinhado a uma parede, para tudo parar sem dizer onde. Agora STL e 3MF eliminam esses triângulos e vedam as frestas de largura desprezível que o gerador de malha do OCCT deixa em faces finas demais; a malha sai fechada e o volume não muda. Furos de verdade continuam abertos e acusados.
- **Verificar sólido** lista cada lasca com posição, largura, face e a operação e o rascunho de origem, e marca o ponto na cena. **Verificar impressão** avisa quantos triângulos saem da malha e onde. O erro de triângulo degenerado também passou a citar as coordenadas.
- Importar STL ignora triângulos de área zero, em vez de recusar o arquivo.
- O STL grava no cabeçalho a contagem real de triângulos e normal nula nos de área zero.

## 0.26.0 — 02/11/2026

- Traços abertos de um rascunho sobre uma face agora delimitam áreas: uma linha que atravessa a face a divide em duas, como se o contorno da face fizesse parte do desenho, sem refazê-lo. Vale para o realce, a extrusão e a pintura, inclusive em bordas curvas e furos. O traço precisa chegar às bordas; um que para no meio não divide nada. Se ele mudar a ponto de a área escolhida trocar de lado, a operação falha com erro.
- **Unir áreas**, em Construção, junta áreas vizinhas no mesmo plano numa só face e tira a linha divisória, sem mudar o volume. Faces em planos diferentes são recusadas.
- Na árvore, arraste um corpo até outra peça para movê-lo, na mesma posição do mundo, ou uma peça até outra para torná-la subpeça. O corpo chega como sólido sem histórico; se a peça de origem depende dele, ele continua lá, oculto.
- Arredondar e Chanfrar em sólidos aceitam vértice: clicar perto dele escolhe todas as arestas que chegam ali.
- Formato `.3dnator` 45: operação `MergeFaces` e `RegionRef::Cell`, a parte de uma face dividida por traços abertos.

## 0.25.0 — 02/11/2026

- Campos flutuantes de medida agora mostram só a caixa do valor. Se o campo perdeu o foco, a primeira tecla o devolve e já entra no valor. O que ele mede e um valor inválido aparecem ao passar o mouse; inválido também deixa a borda vermelha.
- Precisão do mouse passa a 0,4 mm por padrão, com atalhos para 0,2, 0,4, 0,6 e 0,8 e a opção **Personalizado**. Projetos existentes mantêm a sua. Um clique a menos de um passo de um vértice liga nele, evitando falsas conexões muito próximas; Alt solta o cursor.
- Ao desenhar, o cursor captura arestas projetadas e seus pontos médios e, numa linha já iniciada, a tangente a círculos, arcos e elipses, inclusive os projetados do modelo. Um ícone junto ao cursor mostra o que foi capturado e nada fica marcado no desenho. A tangente só posiciona: não cria restrição.
- **Remover vértice**, no botão direito ou com Del, une duas retas numa só aresta. Entre retas colineares as regras da reta mantida continuam; num canto elas saem, porque a direção muda.
- Arredondar e Chanfrar aceitam escolher os cantos por vértice: dá para tratar três cantos de um retângulo. Clicar numa linha volta à escolha por linhas.
- Cruzamento com linha de referência continua imaginário: serve para captura, não oferece **Ligar** e não muda áreas nem o corpo.
- Aparar não perde cliques: os feitos durante o cálculo do corte anterior entram em fila.
- Mover e Copiar: trocar entre Livre e os alinhamentos continua de onde parou. O movimento acumulado segue para o novo tipo, e os cliques valem para a posição exibida.
- Durante Mover, o objeto mostra só as linhas na posição original; ao confirmar, elas somem.
- Projetar mostra só linhas e vértices também durante a escolha, sem preenchimento que esconda o que está atrás.
- Abrir um rascunho oculto não mexe mais no histórico: ele só passa a aparecer se for editado e confirmado.
- Concluir um rascunho de referência volta à aba Montagem.
- Os painéis de ferramentas agora se movem também arrastando o texto do título, não só a área vazia da barra.
- Importar imagem de referência abre na pasta do projeto, como já faziam STL e STEP.
- Soltar vértice deixou de usar cadeado no menu. Ícones de traço de referência e infinito seguem o traço normal, em laranja e com tracejado mais longo.

## 0.24.0 — 02/11/2026

- Com a ferramenta **Extrusão** ativa, só áreas respondem ao cursor: regiões de rascunho, partes de uma face plana delimitadas pelos rascunhos visíveis e faces planas sem desenho. A área sob o mouse é realçada antes do clique; faces curvas e outros elementos não reagem. Clicar de novo na área escolhida a retira.
- Corrigida a extrusão da área externa de um contorno, que era aplicada à face inteira. Clicar fora do contorno agora extruda só essa parte da face, com a área escolhida marcada na cena. A célula acompanha o rascunho no recálculo, e o rascunho fica protegido contra exclusão. Se a área deixar de existir, a operação falha com erro explícito. Com distância zero, a célula divide a face sem mudar o volume.
- A prévia de uma extrusão unida a um corpo não pinta mais o corpo inteiro: destaca só o material novo. Com distância zero, nada é pintado.
- Formato `.3dnator` 45: `ExtrudeFace` ganha `boundaries`, opcional.

## 0.23.1 — 02/11/2026

- Corrigido o erro "A geometria mudou: selecione novamente a aresta/face desta operação" ao editar rascunhos. Faces e arestas com nome repetido ficam guardadas por índice e assinatura dos bytes do B-rep, e qualquer recálculo que mudasse a última casa decimal invalidava a referência, com a geometria no mesmo lugar. Agora o recálculo reencontra a face pelo nome e por um ponto exato sobre ela, e a aresta pelo ponto no meio dela. Sem resultado único, a operação continua falhando com erro explícito, sem escolher outra geometria.
- Confirmar um rascunho que não alimenta o histórico nem uma pintura não recalcula mais a peça.

## 0.23.0 — 02/11/2026

- **Recortar componente** (aba Impressão) abre no corpo a passagem de um componente, inclusive de outra peça: a silhueta na direção de inserção, com a folga aplicada em 2D, do fundo do componente até a abertura. Funciona em STEP real, onde o deslocamento exato do núcleo falhava: 1.259 de 1.259 cortes nos componentes de um projeto real, contra 203 de 370 corpos. A silhueta fica gravada na operação: mudar a folga ou recalibrar recalcula, mas mover o componente pede nova captura. Vãos internos da silhueta ficam preenchidos.
- **Verificar encaixe** compara os corpos como estão posicionados e lista interferências com o volume, peças encostadas e folgas menores que a justa do perfil. O clique marca o ponto na cena. Com uma peça em edição, compara os corpos dela entre si e com o resto; em **Componentes**, um componente com o outro, sem acusar o que há dentro de um módulo importado. A comparação para em 60 s e informa o que ficou de fora.
- **Ressalto** acrescenta ou rebaixa um anel ao longo do contorno interno ou externo de uma face plana, para fechar tampas. O rebaixo usa as mesmas medidas mais a folga, na largura e na profundidade. O lado junto ao contorno usa o próprio contorno da face; o lado deslocado é aproximado por retas a 0,01 mm.
- **Torre de parafuso** ergue torres com furo sobre pontos de um rascunho, com concordância na base. Os presets de autoatarrachante e inserto a quente usam a tabela de fixadores e citam o perfil. Uma torre que não encosta no corpo é recusada.
- **Presilha** cria hastes em balanço com cabeça e rampa sobre pontos de um rascunho, com raio na raiz, e mostra a deformação estimada ao passar a cabeça, avisando acima do limite do perfil. **Janela** usa as mesmas medidas para abrir no outro corpo o vão que recebe a cabeça.
- **Exportar para impressão** ganha **Só um trecho**: uma caixa em coordenadas da peça, desenhada na cena, limita o que sai, para testar um encaixe sem imprimir tudo. O modelo não muda. Vale para corpo ou peça; corpos de malha ficam de fora com aviso.
- **Verificar impressão** mostra a massa estimada e, com preço no perfil, o custo: paredes pela área da superfície e o resto pelo preenchimento. É uma estimativa antes de fatiar, sem suportes. O perfil ganha preço por kg, preenchimento, voltas de parede e deformação máxima.
- Formato `.3dnator` 44: operações `ComponentCut`, `Lip`, `ScrewBoss` e `SnapFit`; novos campos no perfil. Dependência nova: `i_overlay`, para união e deslocamento de polígonos 2D.
- Novos cenários de captura `component-cut`, `fit-check`, `lip`, `screw-boss` e `snap-fit`.

## 0.22.0 — 02/11/2026

- Vértices e arestas do rascunho mostram o estado de restrição de cada elemento, em vez de uma cor para o rascunho inteiro. Vértice livre é vazado, parcial é cheio, restringido é cheio com anel e fixado é um quadrado. Arestas ficam azuis enquanto puderem se mover e verdes quando determinadas. Em conflito, só os elementos envolvidos ficam vermelhos.
- Saíram os selos de ângulo reto, tangência, ponto médio e centro e o texto de junção junto ao cursor. O losango de centro continua.
- A distinção entre junção forte e fraca e o cadeado foram removidos. Capturar vértice, centro ou ponto médio liga; capturar cruzamento ou o meio de uma curva só posiciona. Projetos antigos abrem iguais: ligações fracas não tinham efeito no solver e passam a valer como ausência de ligação.
- Nova seção **Restringir** na aba Rascunho, com Dimencionar, seis ícones (Horizontal/vertical, Coincidência, Tangência, Iguais, Paralelas e Perpendiculares) e a lista completa no rótulo **Restringir ▾**: Normal/Referência, Diagnóstico do rascunho, Fixar/Liberar, Ponto médio, Concentricidade, Colineares, Simetria e Polígono regular. Dá para escolher os elementos antes ou depois: sem a seleção completa, a ferramenta fica à espera dos cliques.
- O botão direito sobre vértice ou aresta mostra só o que vale para o elemento, com Ligar e Soltar vértice. Os dois menus listam o que já afeta a seleção, com × para remover.
- Direção e comprimento são aplicados girando a linha, pela ponta fixada ou pelo meio, em vez de deixar o solver esticá-la. Sem isso, Paralelas numa linha a 56° a deixava 74% mais longa.
- **Coincidência** aceita vértices, arestas e a origem do rascunho, que agora pode ser selecionada e tem marca própria no rascunho. Um vértice vai para a origem, e uma aresta passa a passar por ela. Levar à origem um vértice fixado em outro lugar troca a fixação, sem conflito.
- Tangência passa a valer entre círculos e entre reta e arco sem extremo comum; Iguais, entre círculos e arcos; Horizontal/vertical, entre dois vértices; Concentricidade, em arcos.
- Formato `.3dnator` 43: novas restrições `PointsHorizontal`, `PointsVertical`, `Collinear`, `Symmetric`, `OriginOnCurve`, `EqualRadius` e `CircleTangent`.
- Clicar num vértice seleciona só o vértice, e Ctrl+clique acumula vértices e arestas. Duplo clique no vértice ou F fixa e libera; fixar uma aresta prende os dois vértices.
- Pontas no mesmo lugar sem ligação ganham um ícone de alerta; o clique liga as pontas.
- Novos cenários de captura `restriction-marks` e `restrict-menu`.
- Barra de navegação centrada na base da área de edição, com **Selecionar**, **Medir**, **Home**, **Enquadrar**, **Arrastar** e **Rotacionar**. Arrastar e Rotacionar movem a câmera com o botão esquerdo e suspendem a ferramenta em uso sem cancelá-la: um novo clique devolve o botão a ela. O botão Enquadrar saiu do controle de vista, que mantém o Home; Enquadrar segue no menu do cubo.
- **Medir** abre sem seleção prévia e atualiza o relatório a cada clique no modelo.
- A aba **Inspecionar** foi removida. **Folha técnica** e **STEP** vão para Montagem, **Parâmetros** para o menu do último grupo da Construção e **Corte de seção** para os dois lugares. A folha técnica aberta em Montagem é do componente selecionado; sem seleção, da montagem inteira.
- O nó **Projeto** virou a barra de título da árvore, com um botão que recolhe a árvore para cima. Recolhida, resta só a barra, sobre o canto da área de edição, que ocupa toda a largura.
- A peça em edição saiu do topo da árvore e aparece num balão no topo da área de edição; no lugar do fechar há um ícone de salvar, que conclui a edição e volta à montagem.
- **Selecionar** saiu das abas e fica só na barra de navegação. O item de mesmo nome no menu da aba Rascunho passa a se chamar **Diagnóstico do rascunho** e continua abrindo a situação do solver, a lista de restrições e Reparar rascunho.
- Esc desativa a ferramenta aberta, um nível por vez, e conserva o modo de navegação. Sem ferramenta ativa, Medir, Arrastar e Rotacionar voltam a Selecionar.
- Novos cenários de captura `tree-collapsed` e `navigation-pan`.

## 0.21.0 — 02/11/2026

- **Calibrar** (aba Impressão) exporta uma peça de teste com cinco furos e seis encaixes quadrados de folga crescente, mais um pino. Com as medidas do que saiu impresso, ajusta a compensação de furo e as três folgas do perfil, para qualquer impressora ou fatiador.
- **Furo** volta a ser acessível: a ferramenta existia, mas não aparecia na faixa nem na busca. Agora fica na aba Construção.
- Presets de **Fixação** preenchem o furo para parafuso passante, cabeça embutida, autoatarrachante, inserto a quente e porca cativa. As medidas citam o perfil: recalibrar a impressora ajusta os furos já feitos.
- **Porca cativa** é um novo tipo de furo, com alojamento sextavado. **Furo em gota** troca o topo da seção por uma ponta a 45°, para furos deitados imprimirem sem suporte; a direção fica gravada no furo.
- A tabela de **Fixadores** (M2 a M5 de início) fica no perfil de impressão e pode ser editada, ampliada e guardada na biblioteca. Medidas de insertos variam por fabricante.
- Botões da faixa ficaram 5 px mais estreitos para as 17 ferramentas da aba Construção caberem em 1280 px. Formato `.3dnator` 42: fixadores no perfil, `Nut` e `teardrop` em furos.
- **Verificar impressão** (aba Impressão) analisa o corpo na orientação da mesa e pinta os problemas na própria cena: balanços acima do ângulo do perfil, paredes finas (menos de 2 × bico nas laterais e 2 × camada nas faces horizontais) e frestas menores que o bico. O painel lista cada região com área e medida, e o clique leva a câmera até ela. Também mostra o tamanho na mesa, a área de apoio e avisos de mesa pequena ou apoio insuficiente.
- As análises são estimativas geométricas e valem para qualquer impressora ou fatiador. Em cantos e chanfros, só conta uma superfície de frente, para não confundir parede vizinha com fresta. As cores somem ao fechar o painel e não entram no projeto.
- A exportação para impressão valida corpo a corpo, antes de gravar, citando o corpo e a posição na modelagem. **Montagem inteira** mantém um sólido por corpo: peças encostadas deixam de ser acusadas como defeito, e a posição relativa entre elas é preservada.
- Superfícies abertas, como a serigrafia de placas importadas de STEP, ficam fora da exportação e são informadas. Sólidos com defeito continuam impedindo a exportação.
- Com uma peça em edição e nenhum corpo selecionado, o painel de exportação parte dos corpos dessa peça.
- **Face na mesa** (aba Impressão) escolhe a face plana que fica apoiada, mostra o tamanho resultante e avisa se ele passar da mesa do perfil. A orientação vale só para a impressão: a modelagem não muda, e a escolha acompanha o corpo pelas operações seguintes.
- **Exportar para impressão** grava 3MF ou STL. O 3MF Core traz um objeto por corpo, em milímetros, sem configurações de fatiador. **Preparar para a mesa** aplica a face escolhida, leva a base a Z = 0, centra na mesa e distribui os objetos e suas cópias em fileiras, avisando o que não couber.
- **Abrir no fatiador** usa o aplicativo padrão do sistema ou um comando pessoal, com `{arquivo}` marcando a posição do arquivo. O comando roda sem shell.
- O seletor **O que exportar** ficou compacto, para manter os botões visíveis em janelas menores. Formato `.3dnator` 41: corpos ganham `print_down`.
- Nova aba **Impressão**, com **Perfil de impressão** e **Exportar STL**, que sai de Inspecionar. O perfil usa só grandezas físicas, para servir a qualquer impressora ou fatiador: bico, camada, folgas justa, deslizante e livre, compensação de furo, balanço máximo, densidade e, opcionalmente, o tamanho da mesa.
- Os valores do perfil entram nas expressões de todas as peças como `bico`, `camada`, `parede`, `folga_justa`, `folga_deslizante`, `folga_livre` e `compensacao_furo`. Um parâmetro da peça com o mesmo nome tem prioridade, e mudar o perfil recalcula só as peças que citam o valor alterado.
- A biblioteca pessoal guarda perfis entre projetos; cada projeto mantém sua própria cópia. Formato `.3dnator` 40: novo campo `print_profile`.
- **Reparar rascunho** aponta linhas a menos de 1° da horizontal ou vertical e pontas no mesmo lugar sem ligação. Ao corrigir, só a ponta torta se move: o solver não divide mais o erro entre as duas pontas, o que desalinharia o rascunho de outros que dependem dele.
- **Verificar sólido** avalia a malha na precisão do STL e avisa quando frestas microscópicas colapsariam ao exportar.
- **Verificar sólido** (aba Construção) analisa o corpo e lista contatos em linha: arestas com mais de duas faces, onde dois volumes do mesmo sólido se tocam só ao longo de uma linha. O painel mostra comprimento, posição e a operação em que cada contato surgiu, e o marca no modelo. **Soldar** une os volumes e **Separar** os afasta com um pino de raio ajustável (padrão 0,05 mm), gravado no histórico como **Reparo de contato**.
- O reparo identifica o contato pelos nomes das faces em volta dele: se uma edição o eliminar, a operação falha com erro explícito em vez de agir sobre outra geometria.
- O erro de exportação STL informa quantas arestas têm defeito, a posição da primeira e indica Verificar sólido; ao exportar vários arquivos, cita o arquivo afetado.
- Formato `.3dnator` 39: nova operação `RepairContacts`.
- Extrusões, revoluções e transições voltam a aceitar áreas com folga residual do solver entre pontas sem ligação (até 2e-6 mm): o contorno é emendado antes de chegar ao OCCT, que só conecta arestas a 1e-7 mm e recusava a área com "Não foi possível construir o contorno".
- Trocar o tipo de movimentação na montagem preserva as escolhas de origem/destino e o campo em edição, sem reiniciar o alinhamento.
- Arredondar e Chanfrar aceitam um vértice: as linhas que terminam nele entram na seleção do canto, tanto ao clicar no vértice quanto ao abrir a ferramenta com ele selecionado.

## 0.20.0 — 02/11/2026

- Menu **Abrir** da barra de título reúne a abertura de projeto e os recentes (persistidos entre sessões); **Salvar** abre um menu com Salvar e Salvar como. O seletor de importação começa na pasta do projeto aberto.
- Rascunhos ocultos voltam a ficar ocultos ao confirmar ou cancelar sem mudanças; ao sair da edição a faixa retorna sempre à aba Construção, inclusive para rascunhos de referência. A primeira tecla devolve o foco ao campo flutuante de medidas. O atalho `I` alterna a reta normal e infinita.
- Junções: o botão de cadeado deixa de alternar ligação forte/fraca; o duplo clique no vértice faz essa alternância e o cadeado, junto ao vértice selecionado, fixa ou libera o elemento (`F`). Áreas projetadas deixam de ser preenchidas e mostram apenas contornos e vértices. Os painéis de ferramentas aceitam reposicionamento pela barra de título e o Aparar não abre painel de propriedades.

## 0.19.0 — 02/11/2026

- O botão **Configurações** da barra de título abre um painel da aplicação com as abas **Geral**, **Aparência**, **Desenho**, **Atalho**, **Ajuda** e **Sobre**. As casas decimais exibidas continuam em Desenho e a recuperação automática em Ajuda; as demais abas ficam como esqueleto para a próxima etapa.
- O Loft ganha a opção **Superfície desenvolvível**, ligada em novas transições: de polígono convexo para círculo, cada lado vira um triângulo plano e cada canto um cone oblíquo, sem a torção que impedia a casca. A superfície é exata (o fecho convexo dos perfis) e o círculo continua um círculo. Outros pares de perfis e transições já existentes não mudam.
- A **Casca** passa a funcionar nessas transições, para dentro e para fora, passante ou com fundo: a parede é calculada pelos planos tangentes do corpo, uniforme e com bordas nos planos dos perfis. A face deslocada é poliédrica, com desvio de até 0,005 mm; a superfície original continua exata.
- Novos cenários de captura `settings` e `settings-general` para revisar o painel.
- Formato `.3dnator` 38: `Loft` acrescenta `developable`.
- **Configurações → Geral → Precisão do mouse** define, por documento, o passo linear (0,001 a 1000 mm; padrão 1 mm) e o de ângulos (0,01° a 90°; padrão 1°). No rascunho, o primeiro ponto cai em múltiplos da precisão e cada medida seguinte (comprimento, ângulo, largura, altura, raio e abertura) anda em passos a partir dele, com o valor limpo nos campos dinâmicos. A precisão vale também para arrastar elementos, setas de extrusão, raio, chanfro, casca e deslocamento, Mover (translação e rotação), corte, plano deslocado, imagem de referência e rótulos. Apoios exatos (vértices, centros, curvas e guias) têm prioridade; Alt solta o cursor no rascunho; medidas digitadas aceitam qualquer valor, inclusive intermediário.
- A atração suave à grade do rascunho foi substituída pela precisão do mouse; a grade passa a mostrar múltiplos dela, com pelo menos 20 pontos entre as linhas.
- **Configurações → Geral → Unidade de medida** escolhe mm, cm ou m. Campos, cotas, medições, setas, prévias e parâmetros passam a mostrar a unidade, e o número digitado sem unidade vale nela. A geometria continua em milímetros e o texto gravado leva a unidade explícita (`12 cm`), de modo que trocar a unidade depois não altera o modelo. As casas exibidas aumentam quando for preciso para não esconder a precisão do mouse. Folhas técnicas e arquivos STL/STEP continuam em milímetros.
- Na montagem, a captura em múltiplos de 45° do anel de rotação fica ativa só com precisão de 1° ou mais que passe por 45°.
- A aba **Sobre** exibe ícone, descrição, versão, autor, licença MIT, repositório, formato do projeto e tecnologias; o cenário `settings-about` permite revisão visual.

## 0.18.0 — 30/10/2026

- Os painéis das ferramentas de rascunho passam a usar os mesmos seletores compactos de ícone dos painéis de Mover, Extrusão e Alinhar: traço, modo de linha/retângulo/círculo, modo de deslocamento e filtro de Projetar ocupam uma linha rotulada, com dica por opção no tooltip.
- Medições do rascunho alinham-se à coluna de rótulos dos demais painéis; dicas são pequenas, quebradas na largura do painel e aparecem no mesmo lugar. O painel Projetar ganha botão principal, o painel de imagem usa linhas rotuladas e o Aparar deixa de repetir um botão já selecionado.
- Novos cenários `sketch-line-panel` e `sketch-project-panel` permitem revisar os painéis compactos; o ensaio de modos de desenho foi ajustado ao canvas mais alto.
- Formato `.3dnator` mantido em 37; etapa local sem push/release.

## 0.17.0 — 30/10/2026

- A faixa passa a ter quatro abas fixas, na ordem **Montagem**, **Construção**, **Rascunho** e **Inspecionar**. A antiga aba Peça foi renomeada para Construção; Rascunho continua disponível sem edição ativa e, nesse estado, mantém todo o ferramental visível e desativado, com **Novo rascunho** habilitado.
- Os nomes das seções deixaram de aparecer no rodapé dos grupos. A faixa ficou mais compacta e os grupos continuam separados; as ferramentas secundárias (Selecionar do Criar, Estender e Excluir do Modificar) ficam num botão compacto de menu (chevron) à direita do grupo.
- Novo cenário de captura `rascunho-tab` mostra a aba sem edição ativa. Teste gráfico opcional confere as quatro abas e a ausência dos nomes de seção; a altura uniforme da faixa entre abas continua coberta.
- Formato `.3dnator` mantido em 37; etapa local sem push/release.

## 0.16.0 — 30/10/2026

- Ícones da interface redesenhados sobre a mesma grade de 24 unidades: traço uniforme de 2 unidades, pontas e junções arredondadas, preenchimentos tonais e uma única cor de ação por contexto. A geometria foi simplificada para continuar legível de 16 a 46 pontos, da árvore à faixa e aos painéis.
- Arquivos, modelagem, rascunho, montagem, vista, janela, reprodução da linha do tempo e acessórios passam pelo mesmo sistema visual; o rascunho conserva preto/verde e referências conservam o violeta. O estado desabilitado e a barra escura da janela usam paletas próprias, sem perder contraste.
- Novo cenário de captura `icons` desenha a folha de contato de todos os ícones, com amostra da barra escura; `--render-ui-scenario` aceita largura e altura opcionais para revisão em alta resolução. Formato `.3dnator` mantido em 37; etapa local sem push/release.
- Nenhuma dependência ou recurso gráfico externo foi adicionado: os desenhos continuam vetoriais e próprios do 3dnator.
- Arredondar e Chanfrar do rascunho passam a usar a mesma silhueta de retângulo com o canto superior esquerdo modificado — arco no primeiro, aresta reta no segundo —, legível a 18 pontos e distinta da ferramenta Retângulo.

## 0.15.0 — 30/10/2026

- Confirmar Extrusão com distância zero subdivide as faces coplanares pelo perfil de rascunho. A partição ocorre no B-rep exato, preserva o volume, os furos e a pintura, e aceita várias áreas no mesmo plano. A base é persistida explicitamente; contatos ambíguos ou perdidos e perfis sem uma divisão nova são recusados atomicamente.
- A seta continua disponível em zero mesmo quando ainda não existe divisão válida. A confirmação de zero depende da prévia pronta; dividir, cancelar e desfazer usam as mesmas transações das demais operações.
- Formato interno 37, com leitura de 1–37: a operação usa os campos existentes de Extrude e registra a nova semântica para impedir recálculo por leitores antigos. Regressões de geometria, cores, múltiplas regiões, base ambígua, recálculo, desfazer/refazer, clonagem, persistência, STEP e confirmação pelo mouse.
- Etapa local sem push/release.

## 0.14.0 — 30/10/2026

- Copiar na montagem e na árvore abre o painel com prévia e tipos por ícones: instância compartilhada, cópia independente, padrão linear e circular. A cópia simples aceita os quatro modos de posicionamento, preserva a cor da ocorrência e só cria o componente ao confirmar, em uma transação reversível.
- Modos de movimento por ícones com dicas, no padrão dos rascunhos; alternar face/aresta/ponto/livre preserva o tipo de cópia. Corpos da peça usam cópia independente.
- Pintar recebe o campo de seleção padrão da Extrusão, com contagem e limpeza; limpar também cancela consultas de seleção pendentes. Demais seletores de componentes usam o mesmo componente visual compartilhado.
- Formato mantido em 36; etapa local sem push/release.

## 0.13.1 — 30/10/2026

- Transição guiada retirada da faixa de ferramentas e da busca de criação. Leitura, recálculo e edição de operações existentes permanecem disponíveis para preservar os projetos. Formato 36; etapa local sem push/release.

## 0.13.0 — 30/10/2026

- Grade do rascunho captura cruzamentos, pontos médios de segmentos, centros das células e linhas, com o mesmo passo adaptativo do desenho. A grade cobre os limites visíveis do plano, com limite de linhas para manter a interação responsiva.
- A geometria existente tem prioridade. Guias de alinhamento preservam sua coordenada enquanto a outra pode ser capturada pela grade; Alt suspende a atração. Capturas da grade não criam vínculos permanentes nem fixam o desenho.
- Ensaios de captura em coordenadas positivas/negativas, prioridade geométrica, combinação de guias, desenho real pelo mouse e Alt. Formato mantido em 36; etapa local sem push/release.

## 0.12.2 — 29/10/2026

- Loft alinha costuras circulares aos vértices/direções do outro perfil e compatibiliza círculos em planos com bases diferentes, evitando torção por parametrização arbitrária. Contornos côncavos usam frações de perímetro quando as direções não são monotônicas; círculos permanecem arcos exatos.
- Novas transições de duas áreas usam geratrizes retas por padrão, conforme o comportamento observado no SolidCraft local. A opção continua editável e o modo salvo de projetos existentes é preservado.
- Regressões de simetria retângulo/círculo, cone circular com bases giradas, plano inclinado, furos, recálculo, persistência e STEP. Etapa local sem push/release; formato mantido em 36.

## 0.12.1 — 29/10/2026

- Seleção no viewport da Montagem passa a escolher a raiz do conjunto, incluindo importações STEP, preservando a seleção explícita de componentes internos pela árvore. A importação seleciona suas novas raízes, em vez do primeiro corpo interno. Janela, realce e origem de alinhamento acompanham a hierarquia.
- Ocorrências de filhos compartilhados são identificadas pelo ID e posicionamento; caminhos coincidentes ambíguos exigem a árvore. Testado com o STEP TP4056 indicado pelo usuário (20 peças, 22 instâncias, 151 corpos), movimentos rígidos de todos os descendentes, outra ocorrência preservada e desfazer/refazer.
- Etapa local: 51 ensaios gráficos, 26 testes do documento e ensaio externo aprovados; Clippy com OCCT e formatação aprovados. Sem push/release.

## 0.12.0 — 29/10/2026

- Corrigido o movimento de conjuntos com subpeças: entrar na aba Montagem encerra a edição de corpos, e o movimento atua na instância com toda a hierarquia. Selecionar pai e filho não aplica o deslocamento duas vezes. Pivô e contorno de arraste incluem os descendentes, mesmo quando a peça principal não tem corpo próprio.
- Mover pelo menu de contexto da árvore assume o contexto do elemento escolhido e abre o manipulador para o conjunto. Posições locais dos filhos e outras instâncias da mesma definição permanecem preservadas.
- Modos Livre, Face a face, Aresta a aresta e Vértice a vértice reunidos no painel de propriedades de Mover/Copiar. O botão da faixa abre diretamente o painel. Trocar o modo conserva os alvos e a intenção de copiar; uma face/aresta/vértice de um filho pode alinhar o conjunto selecionado.
- Versão local, sem push/release; formato do projeto mantido em 36.

## 0.11.0 — 29/10/2026

- Campos flutuantes junto ao cursor nas setas de extrusão, raio/chanfro/casca, deslocamento e cantos do rascunho, plano deslocado, seção e eixos/anéis de Mover/Copiar. A medida pode ser digitada durante ou após o arraste, com unidades e vírgula decimal; Enter fixa o valor da prévia e OK conclui a operação.
- Novas setas para ajustar raio de arredondamento, distância do chanfro e espessura da casca em sólidos. Esc no campo restaura o início do gesto; valores inválidos bloqueiam a confirmação e mantêm o campo para correção. Digitar interrompe o ajuste pelo movimento do mouse.
- Foco e edição dos campos permanecem disponíveis durante a preparação da cena. Etapa local, sem push/release; formato do projeto continua 36. Última pendência de `AJUSTES.md` atendida.

## 0.10.0 — 29/10/2026

- Nova ferramenta Rótulo em Acabamento: texto com fontes do sistema, tamanho, negrito/itálico/sublinhado, cor, rotação, posição e arraste sobre faces planas ou curvas; modos superficial colorido, relevo e gravação com profundidade.
- Contornos de letras persistidos no projeto permitem reabrir, mover, redimensionar e recolorir sem a fonte original. Prévia em segundo plano, edição pela linha do tempo e desfazer/refazer. O STEP conserva letras e cores por face no mesmo sólido.
- Cores próprias por face sobrevivem ao recálculo, podem ser substituídas por Pintar e são preservadas com os vínculos de faces dos caches de pintura. Integração adaptativa de volume evita falsos erros ao subdividir superfícies curvas com contornos de texto.
- Formato interno 36; etapa local sem push ou release. Campos flutuantes de medidas permanecem pendentes por solicitação do usuário.

## 0.9.0 — 29/10/2026

- Nova Transição guiada entre perfis: guia automática tangente aos planos, arco por eixo selecionado ou trajetória de linhas/arcos de um rascunho. A seleção gráfica captura a cadeia conectada e o recálculo acompanha suas entidades. Furos conservam correspondência e o núcleo verifica autointerferência.
- Corrigida a seleção de faces sem nome topológico único: usa índice com assinatura exata da geometria. Arestas sem nomes únicos usam distância exata no núcleo; medidas, cotas, realce e ferramentas CAD aceitam essas seleções. Mudanças de geometria exigem nova seleção quando não há identidade estável.
- Perfis finais/adicionais e guia de rascunho são ocultados na criação bem-sucedida das transições/revoluções, preservando escolhas de visibilidade posteriores durante recálculo. Formato interno 35; etapa local sem push ou release.

## 0.8.0 — 29/10/2026

- Nova ferramenta Transição entre perfis: duas áreas em rascunhos de planos diferentes, como base retangular e topo circular, com prévia e opção de geratrizes retas.
- Curvas exatas, sólidos fechados e furos preservados. Correspondências dos furos são capturadas por identidade; substituir um contorno exige atualização explícita. Furos incompatíveis, cruzados ou fora do volume externo são recusados sem publicar resultado parcial.
- Perfis acompanham as edições, ficam protegidos contra exclusão enquanto usados e são remapeados na clonagem. Salvamento/reabertura e exportação STEP preservam a transição. Formato interno 34; etapa local sem push ou release.

## 0.7.0 — 29/10/2026

- Revolução recebe áreas e eixo em campos separados: várias áreas coplanares, inclusive de rascunhos diferentes, e uma linha escolhida diretamente com o mouse. Linhas normais, de referência e infinitas podem servir de eixo.
- A seleção alterna áreas sem apagar o eixo. Perfis e linha acompanham as edições dos rascunhos, são protegidos contra exclusão enquanto usados e têm IDs remapeados na clonagem. Planos distintos são recusados sem alterar o documento; eixos fixos de projetos antigos continuam editáveis.
- Formato interno 33, mantendo leitura dos anteriores. Etapa local sem push ou release.

## 0.6.0 — 29/10/2026

- Dividir recebe Corpo e Plano de divisão. O plano pode ser uma face plana, um plano de trabalho escolhido no viewport/árvore ou um plano da origem selecionado com o mouse.
- Conserva os dois lados como corpos separados, numa ação de desfazer/refazer. O resultado complementar consulta o mesmo plano da divisão primária; editar/mover o plano recalcula ambos. Supressão e exclusão respeitam as dependências, e os dois cartões abrem a mesma edição do plano.
- Planos referenciados ficam protegidos contra exclusão; clonagem remapeia as referências. Formato interno 32 conserva leitura das divisões antigas. Etapa local sem push ou release.

## 0.5.0 — 29/10/2026

- Casca substitui Furo na faixa de criação e na busca; furos existentes conservam histórico e edição.
- Casca oferece Transpassar e seleção independente de uma face de saída do mesmo corpo. Sem saída explícita, raios interiores localizam a primeira face oposta única; corpos com degraus/ramificações pedem seleção quando há ambiguidade. A saída explícita prevalece e pode formar dutos entre faces adjacentes.
- Espessura paramétrica, desfazer/refazer, clonagem e persistência conservam as aberturas. Formato interno 31, compatível com os anteriores; etapa local sem push ou release.

## 0.4.0 — 29/10/2026

- Barra de título com Novo, Abrir, Salvar e Configurações; Salvar oferece Salvar como/Importar/Exportar no menu de contexto. Recuperação e precisão ficam nas configurações.
- Desfazer/Refazer reabrem o contexto da edição, inclusive rascunhos ocultos, sem acrescentar comandos de visibilidade nem apagar Refazer.
- Linha do tempo contextual: montagem lista peças, rascunhos, movimentos, cópias e agrupamentos; rascunho lista geometria, cotas, relações, modificações e projeções; peça inclui rascunhos, planos, pintura, movimentos e cópias entre operações paramétricas. Cartões abrem elementos atuais; os controles de marcador, supressão e ordem de features da peça permanecem disponíveis. Desagrupar remove seu cartão e continua reversível.
- Formato interno 30 persiste referências de atividades e lê versões 1–29. Etapa local, sem push ou release.

## 0.3.1 — 29/10/2026

- Corrigida a cintilação do realce de superfícies ao mover continuamente o mouse em Peça e Pintar. O último destaque válido permanece enquanto a próxima consulta é calculada; mudanças de cena, câmera, enquadramento, seção ou ferramenta descartam respostas antigas. Furos, regiões delimitadas e oclusões continuam respeitados.

- Correção local, mantendo o formato de projeto 29. Sem nova release ou publicação remota.

## 0.3.0-alpha.1 — 29/10/2026

- Primeira pré-release de distribuição: pacote Flatpak Linux x86_64 e instalador EXE Windows x64, ambos com OCCT e versão identificada no aplicativo.
- Compilação e validação dos pacotes pelo GitHub Actions, com verificações de integridade SHA-256 e textos de licença. Corrigidos o nome do pacote OCAF usado pela CI Ubuntu e a consulta de cores XCAF para OCCT 7.6.
- Inclui os refinamentos de rascunho, extrusão, pintura e exportação STEP descritos abaixo. Versão experimental; formato de projeto 29.

### STEP com regiões pintadas e exportação pela árvore

- STEP exporta cores sRGB por face e áreas delimitadas por rascunhos, respeitando furos, interseções, exclusões e a ordem das pinturas. As subdivisões são costuradas numa cópia do sólido; fechamento, validade e volume são conferidos antes da gravação, sem alterar o documento ou criar um corpo por região.
- Exportações por corpo, peça e montagem preservam posições e definições compartilhadas. Pinturas explícitas por superfície e cor de base conservam sua precedência; superfícies sem pintura continuam herdando cores de corpo/ocorrência.
- Menus de contexto de corpos e peças oferecem Exportar STEP e Exportar STL. STEP sugere o nome do elemento; STL abre com o corpo ou os corpos da peça clicada como escopo, mantendo a escolha de qualidade e o resumo. STEP fica desabilitado para corpos de malha.
- Aplicativo 0.3.0 em desenvolvimento e formato interno 29 preservados. STL mantém a geometria original e não carrega cores; a importação STEP no documento continua limitada às cores uniformes de corpo/ocorrência.

### Destaque das áreas de rascunho em Peça e Pintar

- Passar o mouse na seleção de faces da peça ou em Pintar destaca somente a célula delimitada pelos rascunhos visíveis sobre a face, incluindo furos, interseções e a região exterior. O realce usa o mesmo recorte exato aplicado pela pintura; não tinge a face inteira quando há uma delimitação.
- Consulta e recorte em segundo plano, com reaproveitamento da geometria ao mover o cursor dentro da mesma célula. Respostas de outra posição, revisão, peça, câmera ou estado de seção não são exibidas. Corpos à frente e rascunhos ocultos são respeitados. Perfis livres continuam realçáveis na edição da peça; Pintar exige uma superfície de apoio.
- Sem alteração do documento, histórico, cores ou formato interno 29. Aplicativo permanece em 0.3.0 em desenvolvimento.

### Balde de tinta, seleção múltipla e conservação de cores

- Pintar usa ícone de balde e painel com somente superfícies e cores. Cliques alternam superfícies/células; duplo clique seleciona o corpo inteiro, com fila de consultas que conserva cliques durante a atualização da prévia. OK publica todas as escolhas em uma ação de desfazer/refazer.
- Delimitação automática pelos rascunhos visíveis sobre a face: regiões internas/externas, furos e interseções entre vários desenhos, sem campo manual de rascunho. Dois conjuntos de oito cores predefinidas e cor personalizada RGB hexadecimal independente, salva entre ferramentas, projetos e reinícios do aplicativo.
- Corrigida a referência ambígua ao pintar faces com o mesmo nome no histórico: a pintura usa o índice exato com assinatura quando não há um nome único. Corpo inteiro usa uma cor de base que também prevalece sobre a cor da ocorrência, sem depender do número de faces.
- Corrigida a perda de cor após extrudar: o resultado conserva a cor uniforme e pinturas localizadas são transferidas às superfícies herdadas e às faces novas do apoio. Formato interno 29 acrescenta condições de célula e cor de base opcionais, mantendo leitura dos anteriores; aplicativo 0.3.0 em desenvolvimento.

### Extrusão a partir de zero, planos limite e furos do apoio

- Novas extrusões começam em 0 mm, com seta de arraste visível antes de existir um prisma. Zero é um estado transitório sem erro de prévia; a confirmação fica desabilitada até haver comprimento. A seta permanece disponível durante a seleção de várias áreas e as pontas dos dois lados ficam separadas em zero.
- Clicar em áreas de outro plano passa a definir o limite da extrusão, preservando os perfis do plano inicial. Faces planas e planos de trabalho também servem de limite. Rascunhos/planos limite são associativos, persistidos e remapeados na clonagem; sua exclusão é protegida.
- Corrigido o fechamento de furos existentes no corpo ao unir uma extrusão de rascunho sobre sua face. Contornos internos do apoio são prolongados com geometria exata; a saliência ainda pode ultrapassar o contorno externo. O recorte acompanha alterações do furo no recálculo.
- Formato interno 28 acrescenta os limites por rascunho/plano de trabalho, mantendo os limites de face e a leitura dos formatos anteriores. Aplicativo permanece em 0.3.0 em desenvolvimento.

### Pintura de superfícies e refinamento de Projetar

- Pintar na aba Peça, com paleta, seletor RGB e hexadecimal `#RRGGBB`. Faces inteiras ou delimitadas por uma ou várias áreas de rascunho, inclusive projetadas e com furos; prévia, cancelamento e desfazer/refazer. O recorte preciso afeta somente a aparência, preservando o sólido de modelagem, e acompanha alterações do rascunho.
- Projetar reúne as escolhas na lista do painel e permite alternar pela origem, projeção ou lista. Seleção da origem em coordenadas de montagem, independente do plano editado, com profundidade para faces sobrepostas. Finalizar publica toda a seleção em uma revisão; Cancelar/Esc não publica alterações. Arestas paralelas à normal projetam um ponto.
- Imagens podem ser arrastadas diretamente na vista da peça e pelos cantos sem seleção prévia; o gesto conserva o referencial da ocorrência clicada. Durante o desenho, centros e pontos médios aparecem como vértices imaginários, sem alterar a geometria.
- Formato interno 27 salva camadas e cache de pintura e mantém a leitura dos anteriores. A leitura conserva as cores sem OCCT; o recálculo da delimitação exige esse núcleo. Exportação STEP/STL conserva o comportamento anterior. Aplicativo permanece em 0.3.0 em desenvolvimento.

### Áreas, referências e organização da árvore

- Arco acompanha o sentido inicial do mouse e conserva a abertura contínua ao percorrer os quadrantes, inclusive além de 180°. Medidas digitadas respeitam o sentido escolhido; o início não produz uma prévia de volta completa.
- Menus dos rascunhos oferecem editar, renomear e excluir. As pastas Peças/Subpeças, Corpos, Rascunhos e Planos permitem criar elementos pelo contexto e permanecem acessíveis quando vazias. Rascunhos usados pelo histórico continuam protegidos contra exclusão que deixaria referências quebradas.
- Nomes automáticos usam sequências por tipo, independentes de nomes editáveis, quantidade atual e identificadores globais. Novos sólidos recebem Corpo N; refinamentos conservam o nome do corpo de entrada. Contadores persistem após renomear/excluir e são restaurados por desfazer/refazer.
- Projetar inclui áreas, curvas e vértices de outros rascunhos visíveis, faces CAD e áreas de malhas. Áreas coplanares desconectadas da malha são escolhas separadas e diagonais de triangulação não viram contorno. Furos são preservados; geometria projetada continua elegível como perfil de extrusão.
- Extrusão permite alternar várias áreas do mesmo rascunho por cliques, com realce de todas, uma prévia e uma confirmação. Prismas são unidos numa operação, incluindo áreas desconectadas. Nomes de faces incluem uma chave do contorno, para que reordenar as áreas não troque referências do histórico.
- Nova ferramenta Imagem no rascunho: carrega PNG/JPEG, posiciona por dois cantos, oferece largura/altura, posição no plano e leitura da posição na peça, transparência de 0 a 100% (inicial 50%), arraste e redimensionamento pelos cantos. As imagens aparecem no nó Imagens, com visibilidade, edição e exclusão; pixels ficam dentro do projeto e não participam dos perfis.
- Formato interno 26 acrescenta sequências de nomes, imagens RGBA incorporadas e referências de múltiplas áreas, preservando leitura dos formatos anteriores. Aplicativo permanece em 0.3.0 em desenvolvimento.

### Arraste direto, setas de medida e múltiplos cantos

- Retirada de Mover da faixa/busca do rascunho e do atalho M: arestas e vértices são arrastados diretamente na seleção livre, com prévia resolvida em segundo plano, cancelamento por Esc e uma ação de desfazer ao soltar. Ligações fortes transmitem o movimento; apoios fixos recusam destinos incompatíveis.
- Setas com ponta arrastável para Deslocamento, Arredondar, Chanfrar e Extrusão. O simples movimento do cursor não altera distâncias. Os dois lados de Deslocamento têm setas independentes; Enter/OK confirma a operação inteira. Campos numéricos e expressões continuam disponíveis.
- Arredondar e Chanfrar atuam em todos os cantos entre os segmentos selecionados, inclusive em contornos fechados e cantos desconectados. Cortes sobrepostos e ramificações ambíguas recusam a edição inteira.
- Indicadores de centro, ângulo reto, tangência e ponto médio aparecem durante o desenho ou no apoio selecionado. F também alterna a junção do vértice selecionado. Ícones de ferramentas e indicadores usam preto/verde; Arredondar segue o desenho de canto do Chanfrar.

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
