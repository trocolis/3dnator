# Interface desktop

A organização visual usa como referência o shell compacto do SolidCraft e seus estudos em `docs/design/ux-redesign/`: barra escura de documentos, faixa clara de ferramentas agrupadas por tarefa, árvore lateral e histórico na base. Código, desenhos vetoriais e textos desta entrega foram produzidos no 3dnator, sob a licença MIT do projeto.

## Organização e uso

- **Barra superior:** menu Arquivo, salvar, desfazer/refazer, projetos em abas, novo projeto e busca. À direita ficam minimizar, maximizar/restaurar e fechar o aplicativo, substituindo a barra de título nativa. O ponto no nome da aba indica alterações não salvas; o fechamento continua oferecendo salvar, descartar ou cancelar.
- **Modelo:** novo rascunho XY, extrusão, revolução, furo, arredondamento, chanfro, casca, espelhamento, padrão linear e parâmetros. As operações abrem seu editor com a ferramenta já escolhida; perfil e corpo são sugeridos pelo contexto e continuam sujeitos à confirmação.
- **Montagem:** organização de instâncias, ferramentas de repetição e acesso à documentação/exportação. Os controles completos de clones, grupos e padrões permanecem no diálogo de montagem.
- **Inserir:** importação STL/STEP, suporte de referência, rascunhos XY/XZ/YZ ou na face selecionada e exportação. O arquivo próprio continua sendo o meio de preservar o histórico.
- **Inspecionar:** medição da seleção, folha técnica, parâmetros e exportação. Medir exige um elemento capturado na área 3D; os resultados exatos continuam sendo preparados em segundo plano.
- **Vista:** enquadramento, orientações, projeção, sombras, triangulação e visibilidade dos painéis. A pequena faixa sobre a área 3D mantém os atalhos de câmera fora da região de seleção.
- **Rascunho:** aba contextual selecionada ao entrar em edição, com selecionar, linha, retângulo, círculo, arco, aparar e concluir. As cotas, restrições e ferramentas de refinamento permanecem no inspetor rolável. Uma edição pendente precisa ser resolvida ou cancelada antes de concluir.

O rascunho é enquadrado ao abrir, considerando centros e extensões de círculos/arcos. O botão **Enquadrar** da faixa sobre o canvas restaura essa vista após zoom, deslocamento ou redimensionamento da janela. As cotas usam a formatação em português, sem zeros decimais desnecessários.

A árvore usa ícones, controles de visibilidade, nomes abreviados por elipse e grupos por tipo. Duplo clique em peça/corpo entra na construção da peça. Os ícones de pasta e painel no canto da faixa recolhem/restauram árvore e propriedades. Os painéis têm largura ajustável e rolagem; a faixa de ferramentas também rola horizontalmente em janelas menores.

Arraste o fundo livre da barra escura para mover a janela. Duplo clique nessa região maximiza ou restaura; as bordas e os cantos redimensionam a janela quando ela não está maximizada. Os botões da barra recebem seus próprios cliques. Fechar o aplicativo aguarda a tarefa em andamento e segue a confirmação de salvamento dos projetos modificados.

Na base, os ícones do histórico paramétrico abrem a edição das operações da peça ativa. Eles representam operações persistidas, enquanto desfazer/refazer continua pertencendo à sessão. O marcador azul indica o fim da sequência; não é um controle de retrocesso do histórico.

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
- `views.rs`, `sketch_editor.rs` e `navigation.rs`: painéis, canvas e integração com o editor existente.
- `crates/renderer/src/lib.rs` e `view.wgsl`: cor de apresentação do fundo, incluindo o esmaecimento.

A captura do projeto mostra a modelagem 3D. Para revisar a edição 2D, use a captura específica de rascunho. Os dois comandos aceitam dimensões opcionais (900–3840 × 600–2160 pixels):

```sh
cargo run -p nator-prototype --features occt -- --render-project-preview projeto.3dnator modelo.png
cargo run -p nator-prototype --features occt -- --render-sketch-preview projeto.3dnator rascunho.png
cargo run -p nator-prototype --features occt -- --render-project-preview projeto.3dnator compacto.png 900 600
```

As capturas usam a mesma composição egui/wgpu da janela. Elas verificam layout e renderização; não substituem a interação com diálogos nativos. Testes de eventos verificam clique, bloqueio de botões desabilitados, nome acessível e ativação por teclado, além da busca em português sem acento.

A validação Windows está adiada por orientação do usuário. O workflow a mantém disponível por acionamento manual com `validate_windows`, enquanto o trabalho atual prioriza Linux e refinamento funcional.
