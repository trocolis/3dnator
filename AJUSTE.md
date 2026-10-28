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

## Refinar

- [x] A ferramenta Espelhar, deve ter o tipo de traço e duas entradas, uma podem ser selecionada multiplos elementos e a segundo a linha que servira de eixo de espelhamento.
- [x] A ferramenta Deslocamento, deve conter as opções de tipo de traço, e tres tipos, Normal; Dupla, onde recebe duas entradas uma para cada lado; Simétrica onde havera um deslocamento cimetrico para ambos os lados.


## Novo

- [x] Em rascunho, adicionar uma nova ferramenta chama 'Projetar', ele deve projetar Panos, Arestas, Arcos e Vertices de componentes, As linhas devem ser Litaz e prenchimento claro em tom lias também.
- [x] Icones representaivos para desenho, quando um crisamento de linha gera altomativcamente um angulo de 90 graus, deve ter um pequeno icone representando ao lado da junção, o mesmo vale para tanjentes ponto médio e cento, para ilustra para o usuário oque esta ocorrendo.
- [x] Implementar Atalhos, cada contexto de edição deve ter seu próprio grupo de atalhaos: Rascunho (Linha: L, Retangulo: R, Circulo: C, Arco: A, Projetar: P, Apagar: G, Deslocar: S, Aredondar: O, Chanfra: H, Epelhar: E, Mover: M, Dimencionar: D)
