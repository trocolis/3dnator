# Interface desktop

A organização visual usa como referência o shell compacto do SolidCraft e seus estudos em `docs/design/ux-redesign/`: barra escura de documentos, faixa clara de ferramentas agrupadas por tarefa, árvore lateral e histórico na base. Código, desenhos vetoriais e textos desta entrega foram produzidos no 3dnator, sob a licença MIT do projeto.

## Organização e uso

- **Barra superior:** menu Arquivo, salvar, desfazer/refazer, projetos em abas, novo projeto e busca. À direita ficam minimizar, maximizar/restaurar e fechar o aplicativo, substituindo a barra de título nativa. O ponto no nome da aba indica alterações não salvas; o fechamento continua oferecendo salvar, descartar ou cancelar.
A faixa mostra apenas os grupos de ferramentas da aba escolhida, sem bloco fixo de identificação do espaço de trabalho.

- **Montagem:** primeira aba, aberta por padrão, com o fluxo próprio na ordem de trabalho: **Rascunho de referência** (Referência); **Nova peça** ▾ com Criar nova peça, Criar a partir de STEP e Criar a partir de STL (Peças); **Mover** ▾ com Movimento livre, Face a face, Aresta a aresta e Ponto a ponto (Posicionar); **Copiar** ▾ com Nova instância, Cópia independente e Padrão de repetição; e Agrupar/Desagrupar (Organizar). Os botões com ▾ abrem a lista de subopções, cada uma com ícone e uma linha de explicação. Espelhar componente continua no menu de contexto da árvore; folha técnica e STEP ficam em Inspecionar.
- **Modelo:** novo rascunho com seleção de plano, extrusão, revolução, furo, arredondamento, chanfro, casca, espelhamento, padrão linear e parâmetros. As operações abrem seu editor com a ferramenta já escolhida; perfil e corpo são sugeridos pelo contexto e continuam sujeitos à confirmação.
- **Inserir:** importação STL/STEP, suporte de referência, rascunhos XY/XZ/YZ ou na face selecionada e exportação. O arquivo próprio continua sendo o meio de preservar o histórico. Na aba Modelo, Modificar também oferece Combinar, Dividir corpo e Mover/Copiar corpos.
- **Inspecionar:** medição da seleção, folha técnica, parâmetros e exportação. Medir exige um elemento capturado na área 3D; os resultados exatos continuam sendo preparados em segundo plano.
- **Vista:** enquadramento, orientações, projeção, sombras, triangulação e visibilidade dos painéis. A pequena faixa sobre a área 3D mantém os atalhos de câmera fora da região de seleção.
- **Rascunho:** aba contextual selecionada ao entrar em edição, com selecionar, linha, retângulo, círculo, arco, aparar e concluir. As cotas, restrições e ferramentas de refinamento permanecem no inspetor rolável. Uma edição pendente precisa ser resolvida ou cancelada antes de concluir. Ao concluir, a faixa volta à aba de onde o rascunho foi aberto.

**Novo rascunho** entra na seleção de suporte sem modificar o documento. Clique num plano colorido da origem (XY/XZ/YZ), num dos atalhos da faixa sobre a área 3D ou numa face plana do sólido. Cancelar/Esc sai da seleção. Faces curvas e malhas não definem planos de rascunho; a seleção de faces exige OCCT. Os atalhos explícitos de XY/XZ/YZ em Inserir continuam disponíveis.

A edição acontece no mesmo viewport do modelo. A câmera se desloca e alinha ao plano, conservando a projeção; os sólidos continuam desenhados. Grade, entidades, cotas, encaixes e cursor usam o plano local da peça e sua posição na montagem. O cubo, pan, zoom e órbita continuam disponíveis durante a edição. Ao concluir, a câmera retorna à orientação anterior, mantendo o alvo e a escala de trabalho.

A origem do projeto (global) aparece na área 3D por padrão; a origem de cada peça aparece quando o olho de **Origem** dentro da peça está aberto (fechado por padrão), em todas as ocorrências dela. Os planos coloridos só aparecem durante a seleção de suporte. Em uma face, a origem do rascunho é a projeção da origem da peça no plano, com eixos definidos pela normal local; clicar em outro ponto da mesma face não desloca essa referência. O suporte é capturado como plano fixo: ainda não há vínculo paramétrico com alterações posteriores da face.

O rascunho permanece visível ao concluir. Criar uma extrusão, revolução ou furo oculta o rascunho usado, apenas depois de uma operação bem-sucedida. Cada rascunho tem um olho na árvore, inclusive em subpeças, para mostrar/ocultar sem abrir a edição. Clicar no nome abre a edição e mostra o perfil. A visibilidade é salva e participa de desfazer/refazer; recálculos não anulam uma reexibição manual. Projetos anteriores preservam a visibilidade já armazenada.

O rascunho existente é enquadrado ao abrir, considerando centros e extensões de círculos/arcos. O botão **Enquadrar** da faixa sobre o canvas restaura essa vista após zoom, deslocamento ou redimensionamento da janela. As cotas usam a formatação em português, sem zeros decimais desnecessários.

A árvore usa ícones, controles de visibilidade, nomes abreviados por elipse e pastas por tipo (ver **Árvore do projeto**). Duplo clique em peça entra na edição dela. Os ícones de pasta e painel no canto da faixa recolhem/restauram árvore e propriedades. Os painéis têm largura ajustável e rolagem; a faixa de ferramentas também rola horizontalmente em janelas menores.

Arraste o fundo livre da barra escura para mover a janela. Duplo clique nessa região maximiza ou restaura; as bordas e os cantos redimensionam a janela quando ela não está maximizada. Os botões da barra recebem seus próprios cliques. Fechar o aplicativo aguarda a tarefa em andamento e segue a confirmação de salvamento dos projetos modificados.

## Linha do tempo do histórico

Na base fica a linha do tempo das operações da peça ativa, como nos CADs de mercado. Ela representa o histórico paramétrico persistido; desfazer/refazer continua pertencendo à sessão e cobre todas as ações abaixo.

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

- **Pré-seleção:** o elemento sob o cursor é realçado em azul claro antes do clique, com o nome numa etiqueta. Na peça em construção, o modo de seleção das propriedades decide entre face, aresta e vértice; fora dela, a unidade é o componente inteiro. Com um painel aberto, o campo ativo decide (arestas, faces ou corpos), e outras peças não são realçadas. A pré-seleção é desligada acima de 400 mil triângulos na cena.
- **Seleção:** corpos e componentes selecionados ficam tingidos de azul, com arestas vivas azuis, desenhados pela GPU com profundidade. Faces e arestas escolhidas e as referências do painel aparecem realçadas sobre o modelo.
- **Ctrl+clique** acrescenta ou retira um elemento, na área 3D e na árvore. Clicar no vazio limpa a seleção.
- **Janela:** arraste com o botão esquerdo a partir de um ponto livre. Da esquerda para a direita (contorno azul), entram só os elementos inteiramente contidos; da direita para a esquerda (tracejado verde), os tocados. Com Ctrl, a janela acrescenta à seleção. Na montagem, a unidade é a instância; na construção, o corpo da peça ativa.
- **Sobrepostos:** segure o botão esquerdo parado sobre o modelo para listar todos os elementos sob o cursor, do mais próximo ao mais distante. Percorrer a lista realça cada um; o clique seleciona (com Ctrl, alterna). Esc ou um clique fora fecha a lista.

## Importação de STL

O STL não informa a unidade. Depois da leitura, o painel **Inserir malha STL** mostra o nome, a quantidade de triângulos, as dimensões lidas e as dimensões resultantes em milímetros para a unidade escolhida (mm, cm, m, in ou um fator personalizado). OK insere o corpo de malha com a escala aplicada; Cancelar descarta a leitura. A malha continua sendo referência, sem conversão para sólido.

## Peças de outro projeto

**Peça de outro projeto** (aba Inserir) lista os outros projetos abertos em abas e suas definições, com as de topo primeiro e a quantidade de subpeças. OK copia a definição escolhida para o projeto ativo como peças novas e independentes: subpeças repetidas continuam compartilhando a definição copiada e os grupos internos vêm junto. Na construção, a cópia entra como subpeça da peça ativa; na montagem, na raiz. Não há vínculo com o arquivo de origem, que não é alterado. Para repetir uma peça do próprio projeto, use **Nova instância**.

## Unidades, precisão e miniatura

A barra de status mostra a unidade e a precisão de exibição (por exemplo, `mm · 0,00`). O menu permite escolher de 0 a 6 casas decimais para medidas, dimensões nas propriedades, coordenadas e parâmetros. A escolha é uma configuração do documento, salva no projeto e reversível por desfazer; não muda a precisão interna. As cotas dos rascunhos continuam omitindo zeros desnecessários.

Cada gravação inclui uma miniatura isométrica do modelo, mostrada na janela de recuperação para reconhecer as cópias.

## Reparar rascunho

No editor de rascunho, **Reparar rascunho** abre um painel com as correções sugeridas pelo diagnóstico de perfis:

- **Remover entidade duplicada:** mesma linha, círculo ou arco que outra; a mais recente é removida.
- **Remover trecho sobreposto:** linha inteira sobre outra linha colinear que já a cobre.
- **Unir pontas abertas:** extremos soltos mais próximos que a **folga máxima** (0,5 mm por padrão) recebem uma restrição de coincidência; o solver fecha a folga respeitando as demais restrições.

Cada sugestão tem caixa de seleção e aparece em laranja na área 3D; cruzamentos aparecem como × vermelho e pedem **Aparar**, pois exigem escolher o trecho. **OK** aplica só as marcadas pelo mesmo caminho das demais edições (com desfazer); restrições e cotas que dependiam de uma entidade removida saem junto, e pontos isolados de furos são preservados. Nada é corrigido sem confirmação.

## Corte de seção

**Corte de seção** (abas Inspecionar e Vista) esconde a parte do modelo para onde aponta a seta do plano e mostra a seção como tampa hachurada. O plano pode ser XY, XZ, YZ ou o da face escolhida na área 3D (capturado no momento). O deslocamento é digitado ou ajustado arrastando a seta vermelha; **Inverter** troca o lado escondido. OK mantém o corte; Cancelar volta ao estado anterior.

O corte é um controle visual da aba, como isolamento e transparência: não altera geometria, histórico nem desfazer, e não é salvo no projeto. Aparece em **Análises** na árvore, com olho para mostrar/ocultar e menu para editar ou excluir. Seleção, pré-seleção e a lista de sobrepostos ignoram a parte cortada. A tampa é desenhada pelas faces internas do sólido: em malhas abertas, o interior exposto aparece igualmente hachurado.

## Árvore do projeto

A raiz **Projeto** contém **Peças**, **Rascunhos** (de referência da montagem) e **Origem**. Cada peça é um nó com o nome da ocorrência (a definição aparece na dica) e as pastas **Corpos**, **Sub-peças**, **Rascunhos** e **Origem**; cada subpeça repete a mesma estrutura. Pastas vazias dentro das peças não aparecem; as do projeto aparecem sempre.

- **Visibilidade:** todo nó tem um olho. O da pasta não altera a escolha de cada elemento: a visibilidade efetiva combina a do elemento com a de todos os nós acima dele, e o que fica oculto por um nó acima aparece esmaecido. Ocultar **Projeto** esconde tudo; **Peças**, as peças da raiz; **Sub-peças**, as subpeças daquela peça. As pastas são gravadas no projeto, participam de desfazer/refazer e valem para todas as ocorrências da peça. Cena, seleção, medições, miniatura, folhas técnicas e exportações seguem a visibilidade efetiva.
- **Origem:** a do projeto começa visível; a de cada peça começa oculta e, aberta, desenha os eixos em cada ocorrência da peça.
- **Edição da peça:** o lápis ao lado do olho entra na edição da peça (construção), abre o nó dela e leva a faixa da aba Montagem para Modelo; na peça em edição, o lápis fica destacado e um novo clique volta à montagem (e a faixa, de Modelo para Montagem). O nome da peça em edição fica no topo da árvore, com um botão para sair.
- Clique seleciona (Ctrl+clique acrescenta); clique num rascunho abre a edição dele; clicar no nome de uma pasta abre ou fecha a pasta. Grupos, análises e folhas técnicas ficam abaixo da raiz.

## Montagem e posicionamento

- **Rascunho de referência:** escolha um plano da origem ou uma face (o plano é capturado no momento e não acompanha a face). O rascunho pertence à montagem, não a uma peça: é desenhado em violeta, fica na pasta **Rascunhos** do projeto, na árvore (olho, edição por clique, menu com editar e excluir) e nunca serve de perfil para extrusão, revolução ou outra operação. Serve de destino no Mover face a face (plano do rascunho), aresta a aresta (linhas) e ponto a ponto (pontos e extremos).
- **Nova peça:** **Criar nova peça** acrescenta uma peça vazia na raiz da montagem, já ativa para construção, e muda para a aba Modelo. **Criar a partir de STEP** importa cada definição do arquivo como peça nova, com cores e hierarquia; exige o núcleo OCCT, e um executável compilado sem ele mostra o motivo no próprio item. **Criar a partir de STL** abre o painel de unidade e insere a malha como peça nova. Os diálogos aceitam extensões em maiúsculas (`.STEP`, `.STP`, `.STL`). As três desfazem em uma ação.
- **Posição ao lado:** peças novas (vazias, STEP, STL ou de outro projeto), novas instâncias e cópias independentes não aparecem sobrepostas: o componente desliza em +X do mundo até ficar a uma folga das ocorrências visíveis (um décimo da sua maior dimensão, entre 5 e 100 mm). As raízes de um STEP vão juntas, preservando o arranjo do arquivo; uma peça vazia posiciona a origem. Corpos acrescentados a uma peça em construção ficam onde foram lidos, e Mover com **Criar cópia** usa exatamente o deslocamento pedido.
- **Árvore:** cada componente mostra o olho de visibilidade, o lápis de edição e o nome da ocorrência, com a definição na dica. O botão direito abre o menu do componente: ativar, Mover/Copiar, nova instância, clone independente, padrões, espelhar, inserir dentro de outra peça, agrupar a seleção, isolar, transparente, selecionável, ocultar/mostrar, renomear a ocorrência ou a definição e excluir. Corpos têm Mover/Copiar, ocultar/mostrar, exportar STL, renomear e excluir. A seção **Grupos** lista os grupos: o clique seleciona os membros; o menu move, renomeia ou desagrupa.
- **Mover:** o campo Objetos recebe instâncias (montagem) ou corpos da peça ativa (construção), a partir da seleção ou de cliques. Com o campo inativo, o clique escolhe um ponto (vértice, aresta ou face, pelo modo de seleção) que pode virar o pivô. O manipulador aparece no pivô: setas transladam por eixo, anéis giram em passos de 1°, com fixação nos múltiplos de 45° (0°, 45°, 90°, 135°…) a menos de 4°, e o centro move livremente no plano da tela; durante o arraste, o contorno dos elementos acompanha o cursor e a prévia completa é calculada ao soltar. Durante o giro, o ângulo aparece junto ao cursor, na cor do eixo quando fixado. Os campos de translação (mm) e rotação (graus), nos eixos do mundo e com rotação X→Y→Z pelo pivô, são digitáveis: o clique seleciona o valor para substituição e aceitam vírgula ou ponto, unidades e contas (`12,5`, `1 in`, `90 + 45`); texto inválido permanece destacado em vermelho mesmo fora do foco e bloqueia OK/Enter até a correção. Valores fora de ±10.000.000 são recusados sem truncamento. Esc no campo restaura o último valor válido sem fechar o painel; Zerar limpa também os textos pendentes. A origem do padrão circular usa as mesmas regras. O pivô é o centro de massa, a origem do primeiro elemento ou o ponto escolhido. **Criar cópia** gera novas instâncias da mesma definição ou corpos independentes, sem histórico. Vários elementos (por exemplo, todos os membros de um grupo) movem-se em uma só ação de desfazer.
- **Mover face a face, aresta a aresta e ponto a ponto:** abrem o painel de alinhamento já com o tipo escolhido (que ainda pode ser trocado). Clique na geometria do elemento que se move (destacada em azul) e depois na de destino (em violeta), que pode estar em outro componente ou num rascunho de referência; para pontos e arestas, as referências próximas do cursor têm prioridade sobre o modelo. Faces ficam opostas e coplanares no ponto clicado; arestas, paralelas; pontos, coincidentes. **Inverter direção** troca o sentido. Não cria restrição permanente.
- **Copiar:** **Nova instância** repete a definição ao lado da original (editar uma altera todas); **Cópia independente** cria, também ao lado, uma peça nova com histórico próprio; **Padrão de repetição** abre o painel de padrão retangular ou circular.
- **Padrões e espelhamento de componentes:** painéis com o componente, tipo, direção ou eixo, quantidade e espaçamento ou ângulo; o padrão compartilha a definição e reúne as instâncias num grupo. O espelhamento cria uma peça nova e independente.

## Painéis de comando

As operações de construção e de montagem abrem um painel ancorado ao canto superior esquerdo da área 3D (pode ser arrastado): cabeçalho com ícone e nome, campos rotulados e rodapé com o estado da prévia, **OK** e **Cancelar**. Enter confirma e Esc cancela; dentro de um campo em edição, ambos apenas encerram a digitação (Enter conclui o valor), e o próximo Enter confirma o painel com o valor digitado.

- **Campos de seleção** (Perfil, Arestas, Faces, Corpo, Corpo alvo, Corpo ferramenta, Face limite) mostram a quantidade escolhida e um × para limpar. O campo ativo, em azul, recebe os cliques no modelo. Arestas e faces alternam: clicar de novo numa já escolhida a retira. Clicar em outro corpo recomeça a seleção.
- **Perfil na área 3D:** com o campo Perfil ativo, as regiões fechadas dos rascunhos visíveis (e do rascunho escolhido, mesmo oculto) aparecem levemente sombreadas; a região sob o cursor é realçada e o clique a escolhe. Regiões dentro de furos continuam acessíveis.
- **Extrusão:** direção em um lado, dois lados ou simétrica; extensão por distância (com inversão e deslocamento inicial) ou até uma face plana; resultado como novo corpo, união, corte ou interseção com o corpo alvo. Limites gravados fora desses padrões são preservados como estão.
- **Demais ferramentas:** revolução (eixo do plano ou linha do rascunho, ângulo ou volta completa), furo (tipo, extensão, ponta, rosca registrada), arredondamento, chanfro (distância igual ou distância e ângulo), casca (para dentro ou para fora), combinar (unir, cortar, interseção), padrões e espelhamento/divisão por plano. O índice topológico manual fica em **Avançado**.
- **Prévia ao vivo:** a cada alteração, depois de uma breve pausa, o resultado é calculado em segundo plano numa cópia do documento e mostrado no lugar da cena, com o corpo resultante em azul. Ao editar uma operação, a prévia mostra o histórico até ela, e as seleções valem para o modelo antes dela. Erros aparecem no rodapé. **OK** publica o resultado já calculado sem recalcular; se a prévia falhou numa operação posterior, abre-se a decisão de falha no recálculo.
- Seleções obrigatórias vazias desabilitam o OK. A prévia não entra no desfazer nem altera o documento.

## Busca, ícones e acessibilidade

**S** abre a busca de ferramentas; nomes sem acento encontram os rótulos portugueses. Enter executa o primeiro resultado habilitado e Esc fecha a busca. Os atalhos existentes Ctrl+S, Ctrl+Z e Ctrl+Shift+Z/Ctrl+Y foram mantidos. Os campos de texto conservam a entrada normal de letras.

Os ícones são vetoriais, desenhados em uma grade de 24 unidades pelo egui, sem fonte de símbolos ou dependência de imagens externas. Nomes completos ficam nos tooltips e na semântica de acessibilidade, incluindo botões sem texto. Botões aceitam foco e ativação por teclado; ferramentas indisponíveis respeitam o estado de processamento, o contexto da peça e a disponibilidade do núcleo.

O fundo claro do renderizador e do rascunho usa a mesma paleta dos painéis. Cores de seleção, erro, aviso, restrição e medição foram ajustadas para esse contraste. O esmaecimento das peças de referência usa a cor efetiva do fundo, preservando a transparência.

## Cubo de orientação

No canto superior direito da área 3D, clique em uma face, aresta chanfrada ou canto para escolher uma das 26 direções. A área selecionável acompanha o polígono visível, e as faces têm nomes em português que giram junto da orientação. O indicador separado de linhas XYZ foi removido.

Arraste com o botão esquerdo sobre o cubo para orbitar livremente. O movimento continua ao passar pelas vistas superior e inferior; não seleciona nem desloca corpos. A órbita com Shift + botão do meio usa o mesmo referencial da tela.

As duas setas acima do cubo giram a vista no sentido horário ou anti-horário até o próximo múltiplo de 90°. Cliques sucessivos percorrem 0°, 90°, 180° e 270°. Se a vista estiver inclinada pelo arraste, a primeira seta a alinha ao próximo desses ângulos; clicar no número restaura 0°. A direção observada, o alvo, o zoom e a projeção são preservados durante esse giro. Clicar em uma face restaura a orientação convencional dessa vista.

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

Estados interativos também podem ser capturados por cenários (`timeline`, `timeline-menu`, `extrude`, `edit-last`, `failure`, `hover-face`, `hover-edge`, `select-body`, `window`, `assembly-select`, `move`, `align`, `assembly-ribbon`, `section`, `repair`, `recovery`, `insert`, `mesh-import`, `menu-new-part`, `menu-move`, `menu-copy`, `reference-align`, `reference-sketch`, `copy-beside`, `tree-folders`), que não alteram o projeto de origem:

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
