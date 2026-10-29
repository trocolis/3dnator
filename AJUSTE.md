# Lista de BUGs e ajuste detectado apos o teste

## BUGs e Ajustes

- [x] A ferramenta 'Cota' deve mudar de nome para 'Dimencionar'.
- [x] Na ferramenta Cota, se os dois elemento não tiverem posição fixa, o segundo a ser selecionado deve se mover, o primeiro deve ficar fixo. (Regra anterior, substituída pela movimentação inteligente abaixo.)
- [x] Adicionar a possibilidade de editar uma medida feita com Dimencionar.
- [x] Selecionar e movimentar as medidas depois de feitas.
- [x] Movimentação inteligente no dimencionamento, ao selecionar doi elementos e setar a distancia entre eles, fica fixo o elemento com mais conexões e o mais sou se move, em caso de empate o selecionado primeiro se move e o segundo fica fixo.
- [x] No centro do circulo, o icone de centro só deve aparecer se alguma cois estiver ligado a seu centro.
- [x] Em alguma situações a linha das cotas se prolongam por toda a linha de referencia, para evitar isso as linha imaginárias da cota deve paratir do vertice mais proximo.
- [x] Adicionar a ferramenta Mover na seção Modificar de Rascunho.
- [x] Adicionar a ferramenta Chanfra na seção Modificar de Rascunho.
- [x] Mover as ferramentas Espelhar e Dimencionar para a seção Modificar.
- [x] Excluir, por em quanto, as ferramentas Circular e Renatgular da seção Padrão e a propria seção e excluir as ferramentas que sobraram na seção Restringir.
- [x] Em Rascunho, ao apertar Del, excluir elementos selecionados.
- [x] Elementos revovido por Ctrl + Z, continuam como selecionados ao ativar uma ferramenta.
- [x] Na Ferramenta deslocar, deve ser possível deslocar apartir de um segmento inteiro e não apenas de um aresta.
- [x] Ao dar duplo clique em um elemento deve ser selecionado todos os elementos ligados a ele.
- [x] Deve aparecer uma prévia com linhas translucidas em Espelhar e Deslocar, Aredondar e Chafrar.
- [x] O atalho X deve alteranar o traço entre Normal e Construção.
- [x] O Nome do traço Costrução deve ser alterado para 'Referência'
- [x] Um elemento já desenhado pode ser alterado do traço normal para Referência selecionando e apertando X.
- [x] A ferramenta Deslocar deve poder ser dimencionada pelo mouse, com o valor aparecendo ao lado em um fild flutuante.
- [x] O ícone que aparece nos vértices de, de angulo reto, centro e tangente deve aparecer apena em quando se desenha e quando o vértice é selecionado, mantem a tela menos poluída.
- [x] O dimensionamento pelo mouse  do Deslocamento, não esta funcionando bem, altera para uma ceta, e para dimensionar pelo mouse tem que clicar na ponta da ceta e arrastar, algo como a extrusão, mude o da extrusão para uma ceta também, para esse tipo de caso a ceta deve ser o padrão.
- [x] Mudei de Ideia, remove a ferramenta mover da Aba de rascunho, a movimentação de se dar com o mouse, segurando e arrastando as arestas e Vértices.
- [x] Melhore o Ícone da ferramenta Arredondar em Rascunho, deixe no mesmo padrão de  Chanfrar.
- [x] Deve ser possível chanfrar e arredondar vários elementos por vez, onde se formar canto deve ser aplicada a transformação.
- [x] O Chanfro e arredondar deve poder ser dimensionado pelo mouse, segurando e arrastando o indicado
- [x] Padronize a cor dos ícones de dentro em Rascunho com linhas Pretas e verde, o mesmo padrão já presente em alguns ícones.
- [x] Com um vértice selecionado o tipo de junção deve ser alternado pela tecla F também.
- [x] O mesmo padrão de cores deve ser adotado pelos ícones que flutuam ao redor do vértices.
- [x] A ferramenta Arco, o arco esta sendo criado no sentido ante-horário, o sentido deve sere escolido de forma inteligente, ele deve ir do segundo ponta em direção ao movimento do mouse, para dar a volta o mouse deve passeara por baicho do ponto central do arco.
- [x] Os racunhos devem pode ser excluido e renomeados peloa menu de contexto na Arvore.
- [x] A nomeação automática de elementos criados na arvore deve ser o Tipo de elemento seguido do número, exemplo: Corpo 1, Corpo 2, Rascunho 1, Rascunho 2, Palno 1, Peça 1, e o numero deve seguir a ordem de criação, mesmo se 'Rascunho 1' tenha sido renomeado para 'Principal', por exemplo o proximo deve ser 'Rascunho 2', mesmo o 1 não se chamando mais assim.
- [x] Nos nós da árvore, Rascunho, Planos e etc. Deve ser possívem adicionar um novo elemento pelo menu de contexto.
- [x] Os desenhos projetados devem ser levados em conta para formar área de extrusão.
- [x] Deve ser possível selecionar múltipas áreas no mesmo plano para extrusão.
- [x] A Ferramenta Projetar, deve ser possivel selecionar qualquer área, linha ou vertice, e a projeção dele deve ser gerado no plano do rascunho.
- [x] Deve ser possível mover uma imagem clicando nela e arrastando, e redimensionar segurando nos vértices e arrastando.
- [x] Em quanto se desenha deve ser exibido um vértice imaginário nos centos.

## Refinar

- [x] A ferramenta Espelhar, deve ter o tipo de traço e duas entradas, uma podem ser selecionada multiplos elementos e a segundo a linha que servira de eixo de espelhamento.
- [x] A ferramenta Deslocamento, deve conter as opções de tipo de traço, e tres tipos, Normal; Dupla, onde recebe duas entradas uma para cada lado; Simétrica onde havera um deslocamento cimetrico para ambos os lados.
- [x] Ao Iniciar a ferramenta Projetar, em seu painel deve aparece os elementos selecionados, deve ser possível selecionar e desselecionar clicando na projeção e no elemento que a originou, deve ser possível selecionar elementos em qual quer corpo, independente do plano em que ele esta, e a projeção é finalizada ao clicar em Finalizar no painel de propriedades, deve ter também o Botão de cancelar, as áreas, linhas e vértices, geradas pela projeção, devem influenciar na extrusão e pitura .


## Novo

- [x] Em rascunho, adicionar uma nova ferramenta chama 'Projetar', ele deve projetar Panos, Arestas, Arcos e Vertices de componentes, As linhas devem ser Litaz e prenchimento claro em tom lias também.
- [x] Icones representaivos para desenho, quando um crisamento de linha gera altomativcamente um angulo de 90 graus, deve ter um pequeno icone representando ao lado da junção, o mesmo vale para tanjentes ponto médio e cento, para ilustra para o usuário oque esta ocorrendo.
- [x] Implementar Atalhos, cada contexto de edição deve ter seu próprio grupo de atalhaos: Rascunho (Linha: L, Retangulo: R, Circulo: C, Arco: A, Projetar: P, Apagar: G, Deslocar: S, Aredondar: O, Chanfra: H, Epelhar: E, Mover: M, Dimencionar: D)
- [x] Criar uma nova ferramenta para o rascunho, Imagem, pode ser carragada uma imagem para ser usada de referencias, ela deve ser desenhada no Rascunho como o retangulo, ponto inicial e final, ele pode ser redimencionada em seu painel de proproedades, que deve conter tambem o nivel de transparencia, de 0 a 100%, por padrão deve ser 50%, a aposição em relação a origem da peça, e deve poder se arrastada e redimencionada pelo mouse. A imagem deve aparecer na Ravore da peça em um nó Imagens.
- [x] Na aba Peça, criar a ferramenta Pintar, o painel de propriedades deve ter um seletor de cores, com cores predefinidas e um compositor de cores usando o padrão RGB Hexadecimal, '#FFFFFF', a superfície pintada pode ser delimitada por um rascunho.
