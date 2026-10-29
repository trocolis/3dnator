# AJUSTES

- [x] Reorganizar barra de titulo, remover menu Arquivo, ordenar icones, Novo, Abrir, Salvar, Configurações. Este ultimo ainda será definido.
- [x] Refinar desfazer, ao desfazer a ultima ação, se ela for uma edição em um rascuno, deve ser alternado para o modo de edição de rascunho, o mesmo vale para o modo de edição de peças.
- [x] Refinar linha do tempo, A linha do tempo deve ser exibida de acordo com o contexto, Montagem (Criar Rascunho, Criar Peça, Mover Peça, Agrupar Peças), Desagrupara caracteriza como desfazer um agrupamento, então não dev aparecer na linha do tempo; Peça (Criar Rascunho, Extrusão Resolução, Furo, Arredondamento, Chamfro, Pintura, Vover, Copiar, Combinar, Dividir, Criara Plano deslocado), Rascunho (Ajude a definir oque deve ir para alinha do tempo).
- [x] Refinar Resolução, deve haver duas entradas, área usada, podem ser multiplas des de que estejão no mesmo plano; a linha que vai servir de eixo de revolução, deve ser uma linha em um rasculho no mesmo plano, selecionada com o mouse.
- [x] Remova a Ferramenta Furo e a substitua por Casca, Além das opções que já existem no painel de propriedades, Adicione a opção transpaçar, que quando abilitada Cria um duto.
- [x] Adicionar um tipo de extrusão que usa duas áreas em planos diferentes, para criar um corpo, por exemplo, com base retangular e to redondo, fazendo a transforamação de uma forma inicial em outra forma final.
- [x] Adicionar um mode de extrusão que é selecionada duas áreas em planos não paralelos, gerando uma forma curva, o eixo pode ser definido por uma linha em um dos planos, ou altomático. diga se esse modo caba mais como um novo tipo de Revolução?
- [x] A Ferramenta Dividir deve ter duas entradas, o corpo a ser dividido e o plano de divisão, que pode ser uma face do corpo, um plano projetado ou um plano de origem, deve ser selecionado com o mouse.
- [ ] Adicionar uma ferramenta em Acabamento, Para cria rótulo, para adicionar um texto em uma superficie, no seu painel de propriedeade, deve ter o texto, configurações de fonte, tamanho, tipo, negrito, sublinhado, cor e etc, posião, deve ser possível arrastar com o mouse pela superfice, a superfice não perecisa ser plana, usar fontes do sistema, verificar a possibilidade de exportação para step.
- [ ] Toadas as ferramentas que as medidas podem ser ajustadas arrantando a ceta, o valor deve aparecer em um fild flutuante ao lado do cursor, deve ser possível digitar o valor e confirmar apertando Enter, isso deva valer para Extrusão, aredonadamento, chanfro e etc.


## Definições aprovadas

- Casca passante: face de saída selecionável; sem seleção, usar a face oposta quando houver solução geométrica válida.
- Transição curva: aceitar também trajetórias curvas de rascunhos.
- Dividir: conservar os dois lados como corpos separados.
- Rótulo: modos superficial colorido, relevo e gravação.
- Linha do tempo de Montagem e Rascunho: listar e editar, sem reordenação/retorno a etapas neste incremento.
- Trabalhar em etapas com versões e commits locais; sem push ou publicação de release.
