# Validação local do protótipo

## Tipos de linha, ângulo relativo, campos e ícone no cursor — 27/10/2026

- Normal, Construção e Infinita têm ícones e seleção explícita no painel Linha. Segmentos de construção e retas infinitas são tracejados laranja, inclusive fora da edição. A reta infinita é recortada aos limites do canvas; apoios de parametrização não aparecem como extremos nem pontos médios, mas interseções e captura funcionam além deles. Offset, espelho e padrão preservam o tipo.
- `sketch::lines` cobre referência de direção e captura perpendicular (origem, término e cruzamento), recusando atração a segmentos distantes ou ângulos fora da janela. `LineAngle` tem resíduo circular e acompanha a linha de origem no recálculo. Prévia sem referência conserva a horizontal; medidas digitadas têm prioridade.
- Regressões geométricas verificam exclusão de auxiliares das áreas, interseções/captura/seleção da reta infinita, preservação do tipo nas cópias, captura perpendicular e recálculo do ângulo relativo. Regressão CAD extruda o retângulo 20 × 15 mm com diagonal de construção e reta infinita atravessando o perfil: o volume permanece 2.400 mm³ em 8 mm de altura, incluindo desfazer/refazer. Persistência cobre `infinite`, expressão angular relativa e leitura de linha antiga sem o novo campo; formato 18.
- Novo ensaio gráfico desenha uma linha de 12 mm a 90° da referência inclinada, confere a posição dos campos independentes e o ícone vetorial junto ao ponteiro, termina outra linha por captura perpendicular e cria Construção/Infinita pelos botões do painel. Verifica cota relativa, restrição perpendicular, captura longe dos apoios após salvar/reabrir e desfazer/refazer. Campos curtos recebem folga para não se tocar. O ensaio anterior de digitação, Tab/Shift+Tab, Enter/Esc, expressões inválidas e primitivas continua aprovado.
- Workspace testado com e sem OCCT; **65 testes da aplicação aprovados em série, incluindo os 14 ensaios gráficos**. Formatação e Clippy de todo o workspace com todos os alvos/avisos negados aprovados nas duas configurações. Compilação desktop com OCCT e capturas GL/llvmpipe aprovadas no Linux; Windows e janela nativa não exercitados nesta entrega.
- Capturas inspecionadas em `target/codex-validation/line-tools-20261027/`: `normal.png`, `construction.png` e `infinite.png`. Exibem tipo ativo, campos sem caixa compartilhada, prévia angular preta, referência laranja e ícone no cursor. Os cenários `sketch-construction-input` e `sketch-infinite-input` ampliam a captura existente de linha; nenhum altera o projeto de origem.

## Abas espaçadas, pré-seleção de suporte e painéis por ferramenta — 27/10/2026

- Novo rascunho consulta o suporte exato em um trabalhador exclusivo, sem bloquear a interface nem alterar documento ou seleção. A chave inclui aba, revisão, estado visual, seção e raio da câmera; entregas antigas não recebem destaque no alvo atual. O clique respeita a mesma seção e a mesma visibilidade. Planos da origem têm prioridade e apenas um recebe realce quando há sobreposição.
- Duas regressões verificam faces inteiras nas posições compostas de corpo/ocorrência, faces sem nome, bloqueio, tampa plana versus lateral curva de cilindro, malha obstruindo um suporte e entrega já enfileirada de outro cursor/revisão.
- Novo ensaio gráfico escolhe Novo rascunho pela faixa, espera o destaque da face, verifica ausência de alteração do documento e supressão do destaque sobre o cubo, cria o rascunho por clique e confirma que nenhum painel abre automaticamente. Escolhe Linha, Retângulo, Círculo, Arco e Aparar pela faixa, verifica seus painéis e preserva diagnóstico/restrições em Selecionar. Abrir e cancelar Deslocamento não reabre a antiga paleta nem edita o documento.
- Formatação, Clippy de todo o workspace com todos os alvos/avisos negados e testes do workspace aprovados com e sem OCCT. **64 testes da aplicação com OCCT aprovados em série, incluindo os 13 ensaios gráficos**; entrada dinâmica de medidas, aparador, desfazer/refazer e rascunho no mesmo viewport continuam aprovados. Compilação desktop com OCCT aprovada no Linux; Windows e janela nativa não exercitados nesta entrega.
- Capturas egui/wgpu/GL llvmpipe inspecionadas em `target/codex-validation/sketch-tabs-20261027/`: `sketch-idle.png` sem janela automática, `line-settings.png` com painel da ferramenta e medidas dinâmicas e `plane-hover.png` com a face plana destacada durante Novo rascunho. O cenário `sketch-plane-hover` espera uma pré-seleção válida antes de capturar. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Cubo azul em 80%, rótulos, Enquadrar e giros pela normal — 27/10/2026

- Controle reduzido de 208 × 200 para 166,4 × 160 pontos lógicos, incluindo geometria, posições, setas e áreas de clique. Restaurados azuis claros, textos orientados nas seis faces e Home de contorno. Enquadrar ocupa o espaço abaixo de Home; o retângulo de interação do cubo não intercepta o botão nos vazios da silhueta.
- As setas e o menu usam `Camera::rotate_view_step`: 45° em torno da normal assinada da face principal predominante. A normal inicial fica conservada durante uma sequência para evitar troca de eixo quando outra face passar a ser maior. Arraste/presets invalidam a sequência; a base gravada também detecta alterações diretas de yaw/pitch. Alvo, escala e projeção são preservados; arraste continua livre.
- Novas regressões do renderizador cobrem as seis normais, oito passos fechando uma volta, projeções ortográfica/perspectiva, eixo estável na vista oblíqua, reinício após troca de orientação e desempate determinístico. O ensaio de concordância entre matriz GPU, raios, pan e zoom também passa por um giro novo.
- Ensaios do widget verificam dimensões, nomes das seis faces, Home, Enquadrar, giro/inversão das duas setas, menu em 45° e ausência de clique no canvas. O fluxo gráfico de rascunho confirma que Enquadrar usa o plano/perfil ativo e não modifica o documento.
- Capturas reais inspecionadas em `target/codex-validation/view-cube-blue-80-20261027/`: `final.png` (1440 × 900), `compact.png` (900 × 600) e `sketch.png`. O executável desktop foi recompilado. **181 testes sem OCCT e 244 com OCCT aprovados**, além dos doze ensaios gráficos em série (61 testes da aplicação com os ignorados habilitados). Formatação e Clippy do workspace com todos os alvos/avisos negados nas duas configurações aprovados. Validação local no Linux/GL llvmpipe; Windows não exercitado. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Cubo conforme a referência e edição sem barra de câmera — 27/10/2026

- Removida a faixa permanente de vistas/projeção do canvas. O ensaio gráfico da interface compacta verifica a ausência de Ortográfica/Perspectiva/Enquadrar nessa faixa e o espaço recuperado no topo, em 1280 e 900 pixels.
- Navegador com chanfro de 0,60, maior escala, tons de cinza, nomes em dicas, Home preenchido e duas setas curvas preenchidas como na imagem fornecida. A geometria continua fechada com 26 direções; seleção acompanha os polígonos. Faixas das setas usam triângulos explícitos para não preencher o recorte côncavo da ponta.
- Testes do navegador verificam Home com pan/zoom/roll e projeção preservada, passos de 45° nas duas setas, inversão por Shift e botão secundário, menu com troca de projeção/enquadramento/giro de 90°, seleção/arraste sem clique no modelo e tesselação limitada em polos/diagonais/giros. Os giros menores foram transferidos ao menu.
- A posição inicial da paleta de rascunho fica abaixo do navegador; em canvas curto nasce recolhida, preservando posições ajustadas pelo usuário. Capturas reais inspecionadas em `target/codex-validation/view-cube-reference-20261027/`: `final.png` (1440 × 900), `compact-final.png` (900 × 600), `sketch-final.png` e `sketch-compact.png`. A imagem fornecida permanece fora do versionamento.
- **178 testes sem OCCT e 241 com OCCT aprovados**, além dos doze ensaios gráficos em série (60 testes da aplicação com os ignorados habilitados). Formatação, Clippy do workspace com todos os alvos/avisos negados nas duas configurações e compilação desktop com OCCT aprovados no Linux, com capturas via GL/llvmpipe. Windows não exercitado localmente. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Apoios do modelo projetados no rascunho — 27/10/2026

- Identificada a lacuna: ao abrir um rascunho, a interface projetava somente uma amostra de até 64 vértices da malha por corpo, limitada às primeiras oito ocorrências. Os centros e pontos médios geométricos do movimento ponto a ponto não alimentavam essa lista.
- `commands::sketch_projection` passa a consultar os mesmos apoios de corpos e outros rascunhos visíveis, compor as posições do corpo/ocorrência/referencial editado e projetar no plano. O resultado exclui o rascunho ativo e unifica apoios sobrepostos, sem inserir entidades nem vínculos externos. O trabalhador usa chave da cena, plano e posição editada; mudanças descartam entregas anteriores.
- Quatro regressões dos comandos verificam centros de triângulos/retângulos e pontos médios em planos deslocados, projeção inclinada com três posições compostas, a décima segunda ocorrência, visibilidade, fontes auxiliares e geometria CAD exata com alturas sobrepostas. O teste de cache troca plano e bloqueio da ocorrência, rejeitando uma entrega antiga já enfileirada.
- Novo ensaio gráfico usa sólidos em z = 0…6 e rascunho em z = 20: cliques próximos aos apoios criam linhas exatamente no centro retangular, ponto médio da aresta e baricentro triangular projetados. Verifica desfazer/refazer e, ao mover o corpo de origem, recebe o centro novo e remove o anterior do cache.
- **177 testes sem OCCT e 240 com OCCT aprovados**, mais os doze ensaios gráficos em série (59 testes da aplicação com os ignorados habilitados). Formatação, Clippy com todos os alvos/avisos negados nas duas configurações e compilação desktop com OCCT aprovados no Linux. Windows não exercitado localmente. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Aparador entre encontros e referências de polígonos — 27/10/2026

- `curve_geometry` reúne as consultas analíticas antes internas às áreas e compartilha encontros finitos, parâmetros orientados e projeção entre áreas, capturas e aparador. Linhas que só se encontram quando prolongadas não geram vértices; tangências não duplicam nós e círculos coincidentes não inventam um vértice na costura.
- Aparar usa uma cópia candidata, preserva as outras curvas e transforma limites em extremos reais. Regressões cobrem linha/linha, linha/círculo, círculo atravessando 0°/360°, arco horário, tangência, geometria de construção, restrições das entidades intactas, entrada não finita e falha de alocação de identificadores sem edição parcial.
- Captura oferece centros de triângulos e retângulos, inclusive auxiliares/de construção, e metade exata dos arcos. Regressões verificam baricentro distinto do centro da caixa, retângulos inclinados/subdivididos, rejeição de quadriláteros arbitrários/não planos, faces triangulares e laterais de um prisma, as oito quinas do cubo chanfrado e retângulo CAD com furo excêntrico. Malhas coplanares não inventam centros nas diagonais internas.
- Apoios de rascunhos são calculados no trabalhador e invalidados pelas coordenadas/curvas, inclusive sem nova revisão do documento. Apoios de referência da montagem compartilham o cache do movimento ponto a ponto; resultados antigos não são usados por outra revisão.
- Novo ensaio gráfico usa cliques para criar círculos nos centros do retângulo/triângulo, aparar a linha entre os cruzamentos com um círculo e remover uma semicircunferência, verificando desfazer/refazer e salvamento/reabertura. Prévia não altera o documento; captura do centro conserva posição exata, enquanto raio livre admite a precisão da conversão de pixels.
- **173 testes sem OCCT e 235 com OCCT aprovados**, mais os onze ensaios gráficos em série (57 testes da aplicação com os ignorados habilitados). Formatação, Clippy com todos os alvos/avisos negados nas duas configurações e compilação desktop com OCCT aprovados no Linux. Windows não exercitado localmente. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Rascunhos nas quinas de cubos chanfrados — 27/10/2026

- Reproduzido um cubo de 20 mm com as doze arestas chanfradas em 2 mm: seis faces originais, doze faces com o nome repetido de chanfro e oito triângulos de transição sem nome. A seleção de suporte exigia uma referência estável apesar de capturar somente um plano fixo.
- A consulta usa o índice transitório da face na tesselação/B-rep da revisão selecionada e mede o plano antes de compor as posições do corpo e da ocorrência. Nenhum índice é persistido; as regras de nomes estáveis para operações associativas permanecem intactas. Não exige migração nem recálculo de arquivos existentes.
- Nova regressão com OCCT grava/reabre o cubo, seleciona por raios todas as 26 faces com posições e rotações não triviais, verifica planos exatos e cria rascunhos nas oito quinas com desfazer/refazer. Alterar o chanfro para 3 mm, salvar e reabrir preserva os suportes fixos. Regressões existentes continuam recusando faces curvas; testes do documento cobrem consultas sem nome, triângulo inválido e corpo sem B-rep.
- **162 testes sem OCCT e 220 com OCCT aprovados**, além dos dez ensaios gráficos em série (54 testes da aplicação com os ignorados habilitados). Formatação, Clippy do workspace com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados no Linux. Windows não exercitado localmente. Versão 0.3.0 em desenvolvimento e formato 17 preservados.

## Informações no status e diálogo somente para erros — 27/10/2026

- Informações e avisos exibem o texto na barra, com indicador verde/laranja, sem abrir janela ou bloquear atalhos. Pronto/Processando não apagam essas mensagens ao preparar a cena; erros permanecem na fila exclusiva de diálogos.
- A reserva de largura acompanha o texto até 45% do rodapé. Mensagens longas recebem elipse e dica com o conteúdo integral, preservando uma linha e a área da linha do tempo.
- Regressões atualizadas verificam informação e aviso sem diálogo, persistência após transições de processamento, texto longo na barra em 900 pixels, fila de dois erros, confirmação por OK/Esc/Enter e documento intacto. O cenário `status-dialog` passa a representar uma falha.
- Aplicação aprovada com **37 testes sem OCCT e 54 com OCCT**, incluindo os dez ensaios gráficos executados em série. Formatação, Clippy do workspace nas duas configurações e compilação desktop com OCCT aprovados no Linux. Versão 0.3.0 e formato 17 preservados; Windows e janela nativa não exercitados.

## Árvore compacta, rodapé e controle de vista — 27/10/2026

- Removido o cabeçalho da árvore e reduzidas as linhas. A raiz usa o ícone do aplicativo, sem olho; o documento recusa sua ocultação e normaliza a opção legada na leitura, preservando a visibilidade dos descendentes. Regressão existente ampliada para verificar essa compatibilidade.
- Rodapé de uma linha com Histórico, desfazer/refazer, faixa rolável de operações e indicador à direita. Unidade removida; precisão e recuperação disponíveis no menu Arquivo. Mensagens passam por fila tipada e diálogos, sem se perderem quando o estado volta a Pronto. Falhas paramétricas mantêm seu diálogo especializado.
- Navegador conforme o esboço: Home à esquerda, setas de órbita de 45° ao redor e giro da tela em 90°. Ensaios verificam Home após órbita/pan/zoom e roll, preservação da projeção, ângulo das setas, seleção das faces e ausência de cliques no modelo.
- O novo ensaio gráfico confere a barra em 1280 e 900 pixels, fila de avisos/erros, OK/Esc e documento intacto. Enter confirma somente a leitura, sem aplicar um comando Mover pronto por trás do diálogo; um aviso novo não é fechado pelo mesmo Enter que o originou. Capturas reais em `target/codex-validation/compact-shell-20261027/`: árvore do ESP32, linha do tempo e diálogo (`tree.png`, `timeline.png`, `status.png`).
- **162 testes sem OCCT, 219 com OCCT e os dez ensaios gráficos em série aprovados.** Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Capturas finais em `final.png` (1440 × 900) e `compact.png` (900 × 600). Formato 17 e versão 0.3.0 preservados. Validação local em Linux; Windows e janela nativa não exercitados.

## STEP ESP32 com triângulos colapsados — 27/10/2026

- Reproduzida a falha ao importar `esp32_Wroom_30pins_C-Type.STEP` (AP214/SolidWorks, cerca de 14 MiB): a leitura XCAF aceita uma definição com 142 sólidos válidos, mas a malha do corpo de índice 12 contém triângulos com primeiro e terceiro vértices idênticos. A construção estrita de `Mesh` rejeitava a importação inteira com “Triângulo degenerado ou fora do intervalo numérico”.
- `kernel::tessellation` descarta somente triângulos de área exatamente zero da apresentação e compacta seus índices de face junto deles. O B-rep permanece intacto. Três regressões Rust verificam a remoção de elementos repetidos/colineares, a ordem e orientação dos triângulos/índices de face, a preservação de áreas positivas minúsculas e a rejeição de números inválidos e resultados vazios.
- O arquivo real importou **142 corpos e 69.904 triângulos**, com dimensões **28,5 × 53,7 × 13,65 mm**. O ensaio optativo `external_step_preserves_all_solids_after_import_save_and_reopen` passou, comparando quantidade e volume de todos os sólidos, B-reps, mapeamento de faces, salvamento/reabertura, centros, cores e desfazer/refazer. Usa `NATOR_STEP_PATH` para o modelo externo; `NATOR_STEP_PROJECT` opcional conserva o projeto de diagnóstico numa saída ainda inexistente. O arquivo de terceiros não integra o repositório.
- **160 testes sem OCCT e 217 com OCCT aprovados**, além do ensaio externo. Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Projeto e captura 1440 × 900 inspecionada em `target/codex-validation/step-esp32-20261027/` (`esp32.3dnator`, `preview.png`), com egui/wgpu/GL llvmpipe, sem erros de validação gráfica. O STEP original foi apenas lido. Versão 0.3.0 e formato 17 preservados; Windows e janela nativa não exercitados.

## Pontos médios, bordas circulares e centros de arcos — 27/10/2026

- Apoios exatos na metade do comprimento das arestas; em arcos, o ponto médio permanece sobre a curva. Centros virtuais de arcos são descobertos no setor correspondente, no plano da curva, inclusive fora do material. Círculos completos oferecem quatro quadrantes na borda. Todos os apoios usam a captura existente na origem e no destino.
- Malhas oferecem pontos médios somente nas arestas vivas; regressão com retângulo triangulado verifica a exclusão da diagonal interna. Rascunhos de referência oferecem pontos médios de linhas/arcos e quadrantes de círculos, mantendo centro e extremos dos arcos.
- **157 testes sem OCCT e 214 com OCCT aprovados; os nove ensaios gráficos passaram em série.** Regressões novas verificam os doze pontos médios de um bloco, quadrantes internos/externos de uma arruela nas duas alturas, metade do arco em vez da corda, centro virtual do arco no vazio, captura e apoios de referência. O novo ensaio gráfico efetua três alinhamentos entre ponto médio, quadrante e centro de arco, confirma e desfaz/refaz.
- Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Versão de desenvolvimento 0.3.0 e formato 17 preservados. Validação local em Linux; Windows e janela nativa não foram exercitados.

## Centros virtuais no movimento ponto a ponto — 27/10/2026

- Centros exatos de bordas circulares são consultados no B-rep em segundo plano, em coordenadas locais. A projeção compõe corpo e ocorrência, e escolhe a borda frontal quando os centros das duas superfícies ocupam o mesmo pixel. Costuras cilíndricas não geram falsos vértices.
- A região circular descobre o apoio mesmo no vazio do furo; a captura ocorre a até 14 pontos de tela, com mira no centro. Realce e clique usam a mesma consulta, incluindo prioridade dos rascunhos de referência no destino. Ocultação, bloqueio, isolamento, seção e material à frente são respeitados.
- **155 testes sem OCCT e 208 com OCCT aprovados.** As regressões novas verificam alturas e raios de uma arruela, ausência de costuras, descoberta sem interseção com a malha, frente/verso, zoom, vista oblíqua, poses compostas, oclusão, seção, bloqueio, vértices de malha e descarte de consultas de revisão antiga. Os **oito ensaios gráficos passaram em série**; o novo usa eventos egui para escolher origem no furo e destino num disco, verifica a captura do cursor, confirma e desfaz/refaz.
- Formatação, Clippy com todos os alvos e avisos negados nas configurações com/sem OCCT, e compilação desktop com OCCT aprovados. Versão de desenvolvimento 0.3.0 e formato 17 preservados. A validação local usa Linux; Windows e janela nativa não foram exercitados.

## Extrusão por mouse, faces e áreas fechadas — 27/10/2026

- Extrusão direta de face plana preserva contornos e furos, sem rascunho auxiliar. Arraste muda distância e sentido; clique em outra face define o limite automaticamente. Modo automático consulta contato no início e direção de entrada no material: Novo corpo, Unir ou Remover; escolha manual permanece disponível. Remoção mostra a interseção efetiva em vermelho translúcido, sem pintar o trecho do prisma no vazio.
- **153 testes sem OCCT e 200 com OCCT aprovados.** Cinco regressões OCCT novas verificam volumes analíticos de extrusão de faces com/sem furos, modos automáticos e manuais, recálculo por parâmetros, extrusão de células de círculos sobrepostos, limite por plano inclinado, gravação/reabertura e clonagem de referências a faces/laterais/áreas. Regressões Rust cobrem o arranjo de contornos/bordas compartilhadas/ramos abertos, cache assíncrono por revisão e deslocamento inicial/limites dos manipuladores.
- Os **sete ensaios gráficos optativos passaram em série**. O novo escolhe perfil livre (Novo corpo), cancela sem mudar a revisão, escolhe face do corpo (Unir), arrasta para dentro (Remover), verifica a composição de volumes da prévia, clica numa face limite, confirma e desfaz/refaz. Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados.
- Capturas reais `face.png` e `cut.png` inspecionadas em `target/codex-validation/smart-extrusion-20261026/`, com egui/wgpu/GL llvmpipe e sem erros de validação. Cenários `extrude-face`/`extrude-cut`. Formato 17, aplicativo 0.3.0; o único target local instalado é Linux, e Windows/janela nativa não foram exercitados.

## Entrada de cotas durante o desenho — 26/10/2026

- Medidas livres/travadas em tempo real para linha, retângulo, círculo e arco; confirmação por Enter/clique, Tab/Shift+Tab entre campos e Esc para descartar a prévia. Campos aceitam expressões/unidades da peça; erro mantém a etapa aberta e não modifica a revisão. Cotas angulares em radianos, exibidas em graus, com recálculo e salvamento no formato 16; versões anteriores continuam legíveis.
- **150 testes sem OCCT e 192 com OCCT aprovados.** Regressões cobrem prévia com medidas travadas/quadrantes/raio, solução angular em ±π e 180°, expressões angulares com graus/radianos e ida/volta de projeto. Os **seis ensaios gráficos optativos passaram em série**; o novo digita medidas nas quatro formas, alterna por Tab/Shift+Tab, libera o comprimento apagando o campo, verifica atualização pelo cursor, confirma com um Enter, bloqueia raio negativo, cancela por Esc e verifica desfazer/refazer.
- Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Capturas finais de linha, círculo e retângulo (`*-final.png`) em `target/codex-validation/dynamic-sketch-20261026/`, produzidas pelos cenários `sketch-line-input`, `sketch-circle-input` e `sketch-rectangle-input`, sem alterar o projeto de origem. A verificação final da interface está em `app-occt-final.log`: 36 testes, incluindo os seis gráficos, aprovados. Renderização egui/wgpu/GL llvmpipe; aplicativo 0.3.0. Windows e janela nativa não exercitados.

## Faixa Rascunho conforme referência visual — 26/10/2026

- Layout do SolidCraft consultado localmente, com Reference e Configure omitidos. Grupos Criar, Modificar, Padrão e Restringir, grade de seis restrições e Concluir fixo à direita. Capturas 1440 × 900 e 900 × 600 inspecionadas; paleta compacta e recolhida inicialmente em altura menor.
- **146 testes sem OCCT e 188 com OCCT aprovados.** Seis regressões geométricas cobrem edição atômica (incluindo falha parcial por esgotamento de identificadores), padrões/referências, cotas, restrições, tangência e refinamento de cantos. Cinco ensaios gráficos optativos passaram em série; o novo ensaio aplica cota por expressão e deslocamento, desfaz ambos, cancela um padrão sem mudar a revisão e verifica acesso a Concluir em 900 × 600.
- Formatação, Clippy do workspace com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Evidências em `target/codex-validation/sketch-ribbon-20261026/`; renderização egui/wgpu/GL llvmpipe. Versão 0.3.0 e formato 15 preservados; Windows e janela nativa não exercitados. Recursos e limites dos comandos documentados em `INTERFACE.md`.

## Remoção do painel lateral Propriedades — 26/10/2026

- Removidos o painel fixo à direita, o botão/ação que o exibia e os campos de edição associados. A árvore continua recolhível, e o viewport ocupa a largura restante. Controles do rascunho foram transferidos para janela contextual arrastável/recolhível; Medir seleção abre janela própria. O menu de corpo ganhou edição de cor; renomeação, movimento, exportação e exclusão continuam nos comandos existentes.
- **140 testes sem OCCT e 182 com OCCT aprovados**, com os quatro ensaios gráficos optativos executados em série e aprovados, incluindo edição de rascunho, movimentação/alinhamento, cópia e cancelamento. Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados.
- Capturas reais de modelo (1440 × 900), rascunho (1280 × 820) e medição (1440 × 900) inspecionadas, pelo egui/wgpu/GL llvmpipe, sem erros de validação. O cenário `measure` calcula medições do B-rep e mostra dimensões locais e informações da malha na nova janela.
- Evidências em `target/codex-validation/remove-properties-20261026/`. Versão 0.3.0 e formato 15 preservados; Windows e janela nativa não foram exercitados.

## Menu Mover compartilhado por Peça e Montagem — 26/10/2026

- Aba Peça passa a abrir Mover como menu no mesmo grupo Modificar. As opções e suas descrições são compartilhadas com Montagem: Livre, Face a face, Aresta a aresta e Vértice a vértice. O alvo de Livre é corpo na Peça e componente na Montagem. Alinhamentos recusam origens externas durante a edição de uma peça, tanto na seleção quanto na geração do comando.
- Ensaio egui/wgpu ampliado clica nas quatro opções, usa geometria da malha para alinhar o corpo, verifica coincidência no destino, instância sem deslocamento, quantidade de corpos e desfazer. Também verifica que uma origem de outra peça não publica movimento.
- Os **30 testes da aplicação passaram com e sem OCCT**; os quatro ensaios gráficos optativos foram executados em série e aprovados. Formatação, Clippy do workspace com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Capturas reais 1440 × 900 dos menus nas duas abas inspecionadas, pelo egui/wgpu/GL llvmpipe, sem erros de validação.
- Cenário de captura `part-menu-move` disponível. Evidências em `target/codex-validation/part-move-menu-20261026/`. Versão 0.3.0 e formato 15 preservados; Windows e janela nativa não foram exercitados.

## Remoção das abas Inserir e Vista — 26/10/2026

- Removidas as duas abas, seus grupos na faixa e a referência no cenário de captura de inserção. Mantidas Montagem, Peça, Inspecionar e Rascunho contextual; ações continuam na busca por S e nos controles existentes. Os identificadores antigos de tradução ficam reservados.
- Os **30 testes da aplicação passaram com e sem OCCT**; os quatro ensaios gráficos optativos ficaram ignorados nesta execução. Formatação, Clippy do workspace com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados.
- Captura 1440 × 900 da aba Peça inspecionada sem Inserir/Vista; cenário de inserção de outro projeto também executado com sucesso. Evidências em `target/codex-validation/remove-tabs-20261026/`. Versão 0.3.0 e formato 15 preservados; sem execução Windows ou janela nativa.

## Aba Peça e modos Mover/Copiar — 26/10/2026

- Modelo renomeada para Peça pelo identificador de tradução existente. Faixa organizada em Rascunho; Criar (Extrusão, Revolução, Furo); Acabamento (Arredondar, Chanfro, Casca); Modificar (Mover, Copiar, Combinar, Dividir). Espelhamento e padrões continuam na busca; parâmetros continuam em Inspecionar. A definição do item solicitado como “Plano extendido” permanece aguardando esclarecimento.
- Ensaio egui/wgpu novo clica nas duas entradas da faixa, copia um corpo com deslocamento de 100 mm, move a cópia mais 50 mm e desfaz as duas ações. Verifica quantidade de corpos e posições do original e da cópia. Os modos da faixa são fixos; o painel genérico do menu de contexto conserva a alternância existente.
- **140 testes sem OCCT e 182 com OCCT aprovados**, com os quatro ensaios gráficos optativos executados em série e aprovados. Formatação, Clippy com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados. Captura real 1440 × 900 inspecionada, pelo egui/wgpu/GL llvmpipe.
- Evidências em `target/codex-validation/part-ribbon-20261026/`. Versão de desenvolvimento 0.3.0 e formato 15 preservados; Windows e janela nativa não foram exercitados.

## Restauração da cena ao cancelar prévias — 26/10/2026

- Reproduzido o congelamento da cópia após Cancelar: a cena renderizada tinha a geração da prévia, mas a chave da cena original permanecia em `requested` mesmo sem trabalhador ativo. A solicitação agora é liberada ao receber a cena calculada (inclusive obsoleta). Encerrar uma prévia visível solicita outro quadro, pois os painéis fecham depois da verificação da cena.
- Duas regressões sem GPU verificam agendamento ao cancelar explicitamente ou deixar de receber pedidos do painel, retorno ao estado ocioso e cancelamento sem prévia visível. Um ensaio egui/wgpu acrescentado reproduz a cópia no painel, cancela por botão e por Esc e verifica malha/limites restaurados com quadros sem eventos, chave pendente liberada, revisão e salvamento preservados e nenhuma ação de desfazer criada.
- **140 testes sem OCCT e 182 com OCCT 7.9.3 aprovados**, com os três ensaios gráficos optativos executados separadamente em série e aprovados: cancelamento da cópia, digitação no Mover e fluxo de rascunho no viewport. A execução inicial simultânea desses ensaios encerrou o processo com SIGSEGV; a repetição com `--ignored --test-threads=1` passou. Não foi atribuída uma causa nativa sem diagnóstico adicional.
- Formatação e Clippy com todos os alvos e avisos negados aprovados nas duas configurações. Logs em `target/codex-validation/cancel-preview-20261026/`. Sem alteração do formato 15 ou da versão de desenvolvimento 0.3.0; Windows e janela nativa não foram exercitados.

## Corpo modificado sem duplicação na árvore — 26/10/2026

- Reproduzida a causa da apresentação: cada operação mantém seu resultado geométrico no histórico, oculta as entradas consumidas e a árvore enumerava todos esses registros como corpos independentes. A pasta agora usa `Part::listed_bodies`, retirando etapas históricas ocultas e preservando corpos atuais ocultados manualmente e etapas antigas explicitamente visíveis. O contador segue a mesma consulta.
- Modificações passam a herdar o nome da entrada principal; o nome da operação continua independente na linha do tempo. Uma regressão com OCCT confirma que editar uma operação existente conserva a quantidade de resultados, preserva renomeações e recalcula a geometria. Também verifica salvamento/reabertura, recuo do marcador e desfazer/refazer. Outra regressão cobre corpos independentes ocultos, pastas ocultas e etapas antigas exibidas manualmente.
- **138 testes sem OCCT e 180 com OCCT 7.9.3 aprovados**, com dois ensaios gráficos optativos ignorados. Formatação, Clippy do workspace com todos os alvos e avisos negados nas duas configurações e compilação desktop com OCCT aprovados.
- Ensaio `--validate-part` aprovado, incluindo recálculo e intercâmbio STEP/STL. Captura real egui/wgpu/GL llvmpipe inspecionada: o suporte com furo e arredondamento aparece como **Corpos (1)**, com suas operações na linha do tempo. Evidências em `target/codex-validation/body-history-20261026/`. Versão 0.3.0 e formato 15 preservados; Windows e janela nativa não foram exercitados.

## Revisão da montagem e validação de campos — 26/10/2026

- Revisados os três incrementos recentes de montagem e árvore (`3fcaa1c`, `5dda7d0`, `b221ac6`): referências fora das peças, posicionamento ao lado e visibilidade hierárquica. O conjunto existente passou antes das alterações. A revisão identificou que o campo numérico descartava texto inválido ao perder foco e permitia confirmar o último valor válido; valores acima da faixa também eram truncados silenciosamente.
- Corrigida a edição transitória dos campos de Mover e da origem do padrão circular, com validade colhida antes do rodapé e novamente antes de preparar o comando. Quatro regressões novas de eventos egui cobrem perda de foco com texto inválido, bloqueio da confirmação, limites sem truncamento, sincronização externa, clique com seleção do texto, correção incremental e a separação entre Enter/Esc do campo e do painel. Ampliada a verificação de expressões com parênteses, funções e vírgula decimal.
- **137 testes sem OCCT e 178 com OCCT 7.9.3 aprovados**; os dois ensaios optativos de interação gráfica continuam ignorados nesta execução. `cargo fmt`, Clippy do workspace com todos os alvos e avisos negados, nas duas configurações, e compilação desktop com OCCT aprovados.
- Capturas da árvore e do painel Mover em 1440 × 900 inspecionadas, pelo egui/wgpu/GL llvmpipe, sem erros de validação. Imagens e logs em `target/codex-validation/refinement-20261026/`. Não houve execução Windows nem interação com janela nativa. Versão de desenvolvimento 0.3.0 e formato de projeto 15 preservados.

## Rascunhos no mesmo viewport CAD — 26/10/2026

- Removido o canvas 2D separado. A edição, os sólidos, a origem e o cubo compartilham a câmera e o viewport. Regressões verificam ida/volta cursor–plano em XY/XZ/YZ e plano oblíquo, peças rotacionadas, ambas as projeções, recusa de plano de perfil e interpolação rígida incluindo meia-volta. Projetos apenas com rascunhos incluem círculos e pontos isolados no enquadramento.
- Seleção exata de plano pelo B-rep testada com posições distintas de corpo e ocorrência; faces curvas são rejeitadas. A origem/eixos locais de uma face não dependem do ponto clicado nem da orientação da ocorrência.
- **111 testes sem OCCT e 146 com OCCT 7.9.3 aprovados**; formatação e Clippy do workspace com todos os alvos e avisos negados aprovados nas duas configurações. Os testes cobrem ocultação de perfis por extrusão/revolução/furo, falha atômica, reexibição manual, recálculo, desfazer/refazer e salvamento/reabertura, sem alteração do formato 11.
- **Ensaio adicional egui/wgpu aprovado**, optativo por depender de adaptador: cancela a escolha por Esc sem alterar o documento, seleciona XY por clique, desenha retângulo, conclui, extruda, mostra o perfil e escolhe uma face do sólido por clique. Em todos os quadros há exatamente uma imagem CAD com o mesmo identificador de textura; o modelo permanece carregado durante a edição.
- Capturas reais em 1280 × 820 e 900 × 600 inspecionadas, sem erros de validação wgpu/GL llvmpipe: `edit.png`, `edit-compact.png`, `model.png` e `finished-sketch.png` em `target/codex-validation/sketch-viewport-20261026/`. Ensaios CLI `--validate-part` e `--validate-sketch` aprovados nesse diretório. Logs de formatação/compilação/Clippy/testes ficam junto das imagens. Não houve execução Windows nem teste em janela nativa; o suporte de face é um plano fixo, sem associação paramétrica posterior.


## Cubo de orientação e giros da vista — 26/10/2026

- Reconstruído o cubo como poliedro fechado: 26 regiões, 48 arestas compartilhadas exatamente por duas faces e sem invasão dos semiespaços vizinhos. Os cliques escolhem polígonos visíveis, incluindo arestas/cantos, em vez de retângulos sobrepostos. Removido o indicador separado de linhas XYZ.
- Tesselação exercitada em **832 combinações** de azimute, elevação e giros de 90°, incluindo polos e orientações quase coplanares. Todos os vértices permaneceram finitos e dentro dos limites do navegador. O preenchimento usa triângulos, os contornos são segmentos independentes e o controle tem recorte próprio; faces degeneradas são descartadas.
- Eventos egui verificam clique nas faces em ambas as projeções, arraste sem selecionar/mover o modelo e sem encaixe indevido ao soltar, setas percorrendo 0°/90°/180°/270° e retorno a 0° pelo indicador. A câmera cruza os polos com base contínua e ortonormal; seleção, matriz da GPU, pan e zoom no cursor permanecem coerentes após inclinar a tela.
- **105 testes sem OCCT e 137 com OCCT 7.9.3 aprovados** no workspace, com formatação e Clippy sem avisos nas duas configurações. Os 27 testes da aplicação/renderizador foram repetidos após o acabamento dos rótulos e da pintura.
- Compilação desktop Linux e capturas reais egui/wgpu aprovadas em 1280 × 820 e 900 × 600, com imagens inspecionadas: `target/codex-validation/view-cube-20261026/navigator.png` e `navigator-compact.png`. Prévias ortográfica/perspectiva também aprovadas, incluindo cores, arestas, triangulação e transparência, sem erros de validação wgpu/GL llvmpipe. Não foi declarada execução Windows ou interação com janela nativa.

## Controles integrados da janela — 26/10/2026

- Configurada janela sem decoração nativa, com minimizar/maximizar/restaurar/fechar na barra escura superior, conforme esclarecimento do usuário. Capturas reais em 1280 × 820 e 900 × 600 inspecionadas: `target/codex-validation/window-frame-20261026/model-aligned.png` e `compact-aligned.png`, sem erros de validação wgpu/GL llvmpipe.
- Quatro regressões de eventos verificam os comandos de minimizar/maximizar/restaurar; o encaminhamento de fechar ao editor sem forçar saída nativa; arraste/duplo clique no fundo da barra; redimensionamento das bordas e sua desativação ao maximizar. O ensaio detectou e corrigiu a disputa entre o arraste do fundo e o clique dos botões.
- Nove testes da aplicação aprovados com e sem OCCT, incluindo as regressões anteriores da interface. Formatação e Clippy do workspace com todos os alvos e avisos negados aprovados nas duas configurações; compilação desktop Linux aprovada. A execução efetiva dos comandos pelo gerenciador de janelas e a interação com diálogos nativos não são cobertas pela captura sem janela; Windows continua adiado.

## Refinamento da interface — 26/10/2026

- Referência visual: organização compacta do SolidCraft consultada no projeto local. Ícones vetoriais e implementação produzidos no 3dnator. Estrutura e uso em [INTERFACE.md](INTERFACE.md).
- `cargo fmt --all -- --check` e Clippy com todos os alvos e avisos negados aprovados, com e sem OCCT. **91 testes sem OCCT e 123 com OCCT 7.9.3** aprovados. Os testes novos verificam clique, bloqueio de botões desabilitados, rótulo acessível, ativação por teclado, busca portuguesa sem acentos e enquadramento de perfis deslocados/círculos em canvases pequenos.
- Compilação desktop aprovada no Linux. Os seis ensaios de núcleo, projeto, rascunho, montagem/folhas, primeira peça e intercâmbio STEP passaram novamente; relatórios em `target/codex-validation/interface-20261026/validate*/report.json`.
- Capturas reais do editor e rascunho em 1280 × 820 e 900 × 600, pelo egui/wgpu/GL llvmpipe, sem erros de validação. Imagens inspecionadas; a revisão no tamanho mínimo revelou e permitiu corrigir o corte do rascunho por falta de enquadramento. Evidências: `model-final.png`, `compact-final.png`, `sketch-final.png` e `sketch-fitted.png` no diretório do ensaio. Capturas são evidências locais ignoradas pelo Git.
- Prévias ortográfica e em perspectiva do renderizador também passaram, preservando cores, triangulação e transparência (razão de brilho de aproximadamente 0,698). O fundo configurável do editor não alterou o padrão usado pelos ensaios do renderizador.
- Windows foi adiado por orientação do usuário e não foi testado. O workflow mantém a validação nativa como opção manual `validate_windows`; a rotina atual prioriza Linux. Os testes de eventos egui não equivalem a um ensaio de teclado/mouse e diálogos em uma janela nativa.

## Auditoria do plano — 26/10/2026

Situação completa e critérios de aceite em [PLAN_AUDIT.md](PLAN_AUDIT.md). Repetidos `cargo fmt --all -- --check`, Clippy do workspace com todos os alvos e avisos negados, e testes com/sem `occt`: **87 sem OCCT e 119 com OCCT**, todos aprovados. Compilação desktop debug e release aprovada no Linux com OCCT 7.9.3. Não houve alteração de versão do aplicativo ou do formato de projeto.

Cinco regressões de empacotamento Linux aprovadas: destino existente preservado, arquivo comprimido preservado, dependência ausente rejeitada antes da saída, erro compreensível para opção sem valor e execução após mover o pacote e tornar o SDK indisponível. O SDK mínimo do ensaio tem uma biblioteca OCCT com dependência TBB transitiva em caminho com espaços. As duas regressões PowerShell estão preparadas na CI, ainda sem execução local.

O pacote real reuniu 26 bibliotecas OCCT pelo SONAME, sem symlinks, junto com os textos LGPL/exceção. Após movê-lo, os seis ensaios `--validate`, `--validate-project`, `--validate-sketch`, `--validate-editor`, `--validate-part` e `--validate-step` passaram em ambiente limpo. Capturas ortográfica, perspectiva e do editor passaram pelo wgpu/GL llvmpipe, sem erros de validação; o editor e a folha STEP foram inspecionados. Evidências em `target/codex-validation/audit-20261026/`.

O `.tar.gz` release também foi extraído em outro diretório e passou nos ensaios OCCT, da primeira peça e de montagem STEP com ambiente limpo. A verificação de procedência confirmou que os dois textos de licença OCCT estão idênticos, byte a byte, aos arquivos do SDK local.

### Limites numéricos provisórios para a máquina de referência

Os limites abaixo são critérios de aceite inicial, estabelecidos após as medições existentes. Não constituem desempenho comprovado em 8 GB. Usar build release, OCCT, área 3D de 1280 × 800, cinco quadros de aquecimento e 120 de medição, tesselação do suporte com desvio linear de 0,05 mm e angular de 0,25 rad. Rodar três vezes em processos novos, com GPU integrada física e a máquina ociosa; comparar a mediana das três execuções. A abertura mede a leitura do projeto, separada da preparação e do upload gráfico.

| Cenário | Abrir projeto | Quadro p95 | RSS em repouso / pico |
| --- | --- | --- | --- |
| Suporte simples (280 triângulos) | ≤ 250 ms | ≤ 16,7 ms | ≤ 256 / 384 MiB |
| 400 instâncias (112 mil triângulos) | ≤ 500 ms | ≤ 16,7 ms | ≤ 384 / 512 MiB |
| STL de 50 MB (1 milhão de triângulos) | ≤ 2.000 ms | ≤ 33,3 ms | ≤ 512 / 768 MiB |

Importação do STL e análise de topologia: cada etapa ≤ 2.000 ms. Alteração de parâmetro do suporte: p95 ≤ 250 ms, pico RSS ≤ 128 MiB no processo de recálculo. Abertura da janela até o primeiro quadro utilizável: ≤ 5 s, medidos separadamente em três lançamentos com captura de tela/cronômetro. Confirmar que teclado, mouse e cancelamento respondem durante trabalho em segundo plano. No Windows, coletar memória também por ferramenta do sistema, pois o relatório atual só implementa RSS no Linux. Uma medição ausente não aprova o critério.

### Repetição release nesta auditoria

Intel Core i5-11400H, 16 GB, Linux Arch, OCCT 7.9.3; adaptador **llvmpipe/GL (CPU)**, não a Intel UHD. Medição única para diagnóstico, sem aprovar o aceite da GPU integrada. Relatório em `target/codex-validation/audit-20261026/benchmark/report.json`:

| Cenário | Abrir projeto | Quadro médio / p95 | RSS repouso / pico |
| --- | --- | --- | --- |
| Suporte | 0,09 ms | 0,64 / 0,89 ms | 172 / 182 MiB |
| 400 instâncias | 1,32 ms | 16,89 / 17,91 ms | 210 / 254 MiB |
| STL de 1 milhão de triângulos | 364,51 ms | 86,88 / 94,40 ms | 431 / 522 MiB |

Importação STL: 270,45 ms; topologia: 306,23 ms; recálculo p95: 21,08 ms; pico do processo de recálculo: 37,5 MiB. Os quadros de instâncias e da malha excedem o objetivo numérico sob renderização por software; isso fica registrado e não é convertido em uma alegação de aprovação no hardware de referência. O buffer gráfico atual usa 96 bytes por triângulo, incluindo a cor.

## Continuação 0.3.0 — empacotamento, distribuição e licenças (26/10/2026)

- Estratégia de distribuição e empacotamento documentada em `docs/DISTRIBUTION.md`, estabelecendo a arquitetura de pacotes portáteis autocontidos e os scripts automatizados `scripts/package_linux.sh` e `scripts/package_windows.ps1`.
- Ensaio de empacotamento no Linux: o script reuniu o executável ELF, resolveu automaticamente as dependências dinâmicas via `ldd` copiando 78 bibliotecas e symlinks do OCCT 7.9.3 para `lib/`, e gerou o launcher `3dnator` com `LD_LIBRARY_PATH` local.
- Execução do pacote portátil em ambiente estritamente limpo (`env -i PATH=/usr/bin:/bin`): verificado que `./3dnator --version` identifica corretamente "3dnator 0.3.0 | OCCT: habilitado" e que o ensaio `--validate` completa com sucesso todos os passos de B-rep, corte, furo, arredondamento e intercâmbio STEP/STL, confirmando que a distribuição opera de forma independente e autocontida.
- Inventário de licenças em `THIRD_PARTY.md` enriquecido com o aviso proeminente e texto integral da *Open CASCADE Technology Exception (version 1.0)* à GNU LGPL 2.1, instruções para obtenção de fontes e garantia de substituição de bibliotecas dinâmicas pelo usuário final.

## Continuação 0.3.0 — cortes precisos do B-rep (26/10/2026)

83 testes sem OCCT e 113 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. Uma seção anular no núcleo preserva os dois contornos, a área esperada dentro da tolerância de amostragem e o volume do sólido original. Outra regressão impede que sobreposições de corpos distintos cancelem suas hachuras. Vistas e traços dos planos usam a interseção preparada uma única vez por corte; curvas de HLR e corte compartilham amostragem por intervalos C2 com desvio de 0,01 mm. O ensaio `--validate-step target/codex-validation/precise-sections-001` gerou PNG/PDF com cortes precisos A–A/B–B da montagem posicionada. A legenda identifica **HLR E CORTES DO B-REP**; contextos com malhas continuam aproximados. O formato de projeto permanece em 10.

## Continuação 0.3.0 — múltiplos cortes e formato da folha (26/10/2026)

82 testes sem OCCT e 111 com OCCT aprovados; formatação e Clippy aprovados nas duas configurações. As regressões verificam manutenção de identificadores após remoção de A–A, retrato/paisagem, dimensões do PNG na resolução escolhida, rejeição de cortes repetidos ou inválidos e salvamento/reabertura/desfazer/refazer das configurações. `--validate-editor target/codex-validation/multiple-sections-001` aprovado, com folha A4 em retrato, cortes A–A/B–B perpendiculares e PNG em 96 DPI. Projeto, PNG e PDF estão no diretório do ensaio; a imagem foi inspecionada. O formato de projeto passou a 10, com cortes adicionais vazios, paisagem e 150 DPI por padrão em folhas anteriores.

## Continuação 0.3.0 — vistas precisas do B-rep (26/10/2026)

79 testes sem OCCT e 108 com OCCT aprovados; formatação e Clippy com avisos negados aprovados nas duas configurações. As regressões novas confirmam que um bloco atrás de outro recebe linhas ocultas, que o perímetro visível externo confere com o analítico e que a silhueta circular é extraída do B-rep com controle explícito de desvio. A folha de um cilindro com malha propositalmente grosseira usa HLR; acrescentar um corpo de malha troca para o método aproximado identificado na legenda. O ensaio `--validate-step target/codex-validation/step-hlr-001` também gera `drawing.pdf` e `drawing.png`, com quatro vistas HLR da montagem, escala 0,5 e linhas ocultas. A imagem foi inspecionada. Cortes ainda são calculados sobre a malha e isso permanece explícito na legenda.

## Continuação 0.3.0 — montagem STEP (26/10/2026)

- 79 testes sem OCCT e 105 com OCCT 7.9.3 aprovados. Três regressões novas verificam nomes UTF-8, definições compartilhadas por raízes e subpeças, cores de corpo e ocorrência, posições rotacionadas, persistência sem o STEP de origem, exportação/reimportação, atomicidade e rejeição de ciclos e malhas no escopo de exportação.
- `--validate-step target/codex-validation/step-001` aprovado: bloco de 6.000 mm³, duas ocorrências com cores distintas e montagem posicionada. O arquivo de origem foi removido antes da reabertura e da segunda exportação. Projeto resultante em `imported.3dnator`, STEP em `assembly.step` e relatório em `report.json`.
- A ponte XCAF foi compilada localmente com o SDK 7.9.3. A CI Linux/Windows recebeu o ensaio; execução com OCCT 7.6 e no Windows permanece pendente. Superfícies abertas, referências externas e cores por face não foram declaradas como suportadas.

## Continuação 0.3.0 — cores sólidas (26/10/2026)

79 testes sem OCCT e 102 com OCCT aprovados. Formatação e Clippy com avisos negados aprovados nas duas configurações. A regressão nova verifica comando de cor, cena, clonagem, salvamento/reabertura e desfazer/refazer. A captura `target/codex-validation/colors-001.png` verificou por leitura real de pixels 31.727 pixels da cor de teste, além de triangulação e transparência. Formato de projeto 8; projetos antigos permanecem com a cor neutra. O buffer passa de 84 para 96 bytes por triângulo; medições históricas de memória anteriores a essa mudança não foram repetidas.

## Continuação 0.3.0 — espelhamento de conjuntos (26/10/2026)

- Testes do workspace: 78 aprovados sem OCCT e 101 com OCCT 7.9.3. Formatação e Clippy com avisos negados aprovados nas duas configurações.
- Novas regressões cobrem planos oblíquos nas coordenadas de um pai rotacionado, corpos posicionados localmente, definições compartilhadas por vários caminhos, grupos internos, ocorrências e corpos finais ocultos, salvamento/reabertura, duplo espelhamento, desfazer/refazer e preservação do documento e do refazer após falhas.
- A rotação da imagem mantém determinante +1. Casos em ±90° verificam também a correção da conversão de matrizes para ângulos, que perdia precisão perto da singularidade.
- Com o núcleo real, verificados volume, área e centro de massa de três ocorrências sólidas em um conjunto com corpo de malha. Alterar a espessura da subpeça espelhada de 8 para 15 mm depois de reabrir recalcula seus corpos visível e oculto, mantendo as definições originais independentes.
- Ensaios `--validate-editor` e `--validate-part` aprovados; relatórios e arquivos em `target/codex-validation/assembly-editor-001/` e `assembly-part-001/`, respectivamente. O ciclo da peça conserva volume analítico, referências, persistência e intercâmbio STEP/STL.
- Validação Windows e interação manual com o diálogo de montagem continuam pendentes. O formato do projeto permanece na versão interna 7.

## Continuação 0.3.0 — perspectiva (26/10/2026)

- `cargo fmt --all -- --check` e Clippy do workspace com avisos negados, com e sem OCCT: aprovados.
- Testes do workspace: 73 aprovados sem OCCT e 95 com OCCT 7.9.3. A câmera é verificada nas duas projeções quanto a zoom no cursor, alteração dos limites da cena, correspondência entre seleção/anotações e matriz da GPU, profundidade wgpu [0, 1], enquadramento e movimento livre.
- Capturas reais pelo wgpu, backend GL/llvmpipe: perspectiva com 31.763 pixels de superfície e 1.564 pixels alterados pela triangulação; ortográfica com 39.690 e 1.644, respectivamente. O estilo transparente reduziu o brilho para aproximadamente 69,8% nas duas projeções, sem erro de validação wgpu.
- Evidências locais: `target/codex-validation/perspective-001.png`, `orthographic-001.png` e `editor-001.png`. As capturas não validam interação manual ou desempenho de GPU física.

## Incremento 0.3.0 com OCCT — 26/10/2026

Validação em Linux x86_64 (Arch, Rust 1.99.1, GCC 16.2.1) com OCCT 7.9.3 extraído do pacote oficial do Arch, checksum conferido com o banco de dados assinado do pacman. Primeira execução da ponte C++ com um SDK real.

| Verificação | Resultado |
| --- | --- |
| Configuração CMake da ponte | Falhava em qualquer SDK 7.x pela ordem dos componentes; corrigida |
| `cargo clippy --workspace --all-targets` com e sem `occt` | Aprovado, avisos negados |
| Testes do workspace com `occt` | 90 aprovados (68 sem `occt`): nomeação e operações mecânicas no núcleo; fluxo paramétrico, furos, padrões, espelhamento, divisão, montagem espelhada, medição exata e qualidade de exportação nos comandos |
| `--validate` (primitivas, corte, furo, arredondamento, chanfro, erro preservando a origem, tesselação, STEP/STL) | 9 passos aprovados; 37 ms; pico RSS 46 MiB |
| `--validate-project`, `--validate-sketch`, `--validate-editor` com o binário `occt` | Aprovados |
| `--validate-part` (critério de entrega da etapa 2) | Aprovado: volume final 14.375,365 mm³ igual ao analítico; STEP reimportado com o mesmo volume e caixa 60 × 25 × 30 mm; STL fechado e orientado, 656 triângulos, desvio de volume de 0,001% |
| Janela real em Wayland (GPU Intel UHD/NVIDIA) | Abre e permanece estável por 20 s; interação não testada |
| Windows, Ubuntu (OCCT 7.6) e diálogos nativos | Pendentes; workflows preparados |

### Medições de leveza (`--benchmark`, release)

Notebook com Intel Core i5-11400H, 16 GB de RAM, GPU integrada Intel UHD (Tiger Lake GT1, Mesa 26.2.5, Vulkan) e GPU dedicada GeForce GTX 1650. A integrada é a referência do ROADMAP e é escolhida por padrão. Cada cenário roda em processo próprio, com a área 3D em 1280 × 800 e 120 quadros de órbita contínua; o tempo de quadro inclui a espera pela GPU.

| Cenário | Triângulos | Abrir projeto | Quadro médio / p95 (integrada) | Quadro médio (dedicada) | RSS em repouso / pico (integrada) |
| --- | --- | --- | --- | --- | --- |
| Peça simples (suporte OCCT com furo e arredondamento) | 280 | 0,1 ms | 0,66 / 0,74 ms | 0,06 ms | 146 / 148 MiB |
| 400 instâncias da peça | 112 mil | 1,0 ms | 3,71 / 5,01 ms | 0,15 ms | 151 / 159 MiB |
| Malha STL importada (toro, 50 MB) | 1 milhão | 330 ms | 8,56 / 9,15 ms | 1,43 ms | 232 / 312 MiB |

- Importar o STL de 1 milhão de triângulos leva 268 ms, e a análise de topologia e arestas vivas, 272 ms. O projeto resultante tem 6,5 MB.
- Recalcular o histórico do suporte (rascunho, extrusão, furo, corte e arredondamento) leva 20,7 ms por mudança de parâmetro; a construção inicial, 27 ms.
- Cerca de 145 MiB do RSS em repouso são o processo com o driver Vulkan e os buffers da GPU integrada, que usa a memória do sistema.

Antes das otimizações deste incremento, a mesma malha de 1 milhão de triângulos tinha pico de 575 MiB, abria em 507 ms e levava 211 ms para preparar os dados da GPU. As mudanças foram:

- remover o buffer de linhas da triangulação: as linhas agora são desenhadas no shader por coordenadas baricêntricas, o que eliminou 144 bytes por triângulo;
- fundir vértices idênticos na importação STL e na tesselação: a malha passou de 72 para cerca de 12 bytes por triângulo na CPU e no arquivo;
- calcular as arestas vivas uma vez por geometria, em vez de a cada exibição;
- gravar os vértices da GPU sem uma alocação por vértice.

Esses números valem para uma máquina mais forte que a referência de 8 GB com GPU integrada. A medição na própria máquina de referência ainda está pendente.

O cenário clássico de nomeação topológica foi verificado: o arredondamento do canto interno do suporte permanece na mesma aresta quando a espessura muda de 6 para 9 mm, e o volume confere com (1 − π/4)·r²·L. Substituir o canto por uma diagonal faz o recálculo falhar com "Referência perdida", sem aplicar o arredondamento em outra aresta e sem alterar o documento.

## Incremento 0.2.0 — 26/10/2026

Validação em Linux x86_64 com as dependências do cache, sem SDK OCCT ou servidor gráfico acessível.

| Verificação | Resultado |
| --- | --- |
| Formatação e Clippy do workspace, com avisos negados | Aprovados |
| Testes do workspace | 22 aprovados |
| Ciclo CLI de projeto com dois corpos | Aprovado, incluindo reabertura após remover o STL original |
| Posições de corpo/instância | Preservadas, sem alterar vértices locais |
| Salvamento, desfazer/refazer e recuperação separada | Aprovados nos ensaios e testes de persistência |
| Limpeza de recuperação | Projetos normais preservados mesmo na mesma pasta |
| Resultado antigo de outra aba/revisão | Rejeitado no teste do controlador |
| Interface real em egui/wgpu, sem janela | Capturada em 1280 × 820, sem erro de validação wgpu |
| Diálogos nativos e interação em janela | Pendentes de ambiente gráfico |
| Windows e integração OCCT | Pendentes de execução; workflows preparados |

Evidências locais, ignoradas pelo Git:

- `target/project-validation/run-003/report.json`: oito verificações do ciclo de projeto, resultado `passed`.
- `target/project-validation/run-003/support.3dnator`: uma peça com dois corpos; a instância foi deslocada 100 mm em X e o segundo corpo 30 mm em Y dentro da peça.
- `target/project-validation/run-003/recovery.3dnator`: snapshot posterior de recuperação, separado do salvamento manual.
- `target/project-validation/editor-0.2-final.png`: tela produzida pela composição real do aplicativo, carregando o projeto e mostrando uma aba auxiliar.

O adaptador da captura foi llvmpipe, via backend OpenGL do wgpu. Não foi realizado benchmark da máquina de referência de 8 GB/GPU integrada. A etapa de edição paramétrica ainda não foi implementada.

Os testes novos também cobrem transformação rígida, amplificação de vértices por instâncias, limites do documento, compartilhamento de geometria entre snapshots, falhas atômicas e marcação correta de um salvamento que termina após outra edição.

## Registro histórico do protótipo 0.1.0

Registro de 25/10/2026. O teste local usa Linux x86_64, Rust 1.99.1 e as dependências disponíveis no cache, sem acesso de rede pelo terminal.

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --all -- --check` | Aprovada |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Aprovada sem avisos |
| `cargo test --workspace --offline --locked` | 9 testes aprovados |
| `cargo build -p nator-prototype --offline --locked` | Aprovada para Linux |
| Exportar e reimportar suporte em STL | 60 × 20 × 30 mm, volume 10.080 mm³, 20 triângulos, sem arestas abertas ou inconsistentes |
| Captura headless pelo wgpu | Aprovada; superfícies e sobreposição de triangulação exercitadas |
| Checagem de tipos e Clippy do caminho Rust com OCCT | Aprovada por emissão de metadados, incluindo o código desktop; não compila nem vincula C++/OCCT |
| Compilação com `--features occt` | Bloqueada: SDK e `OpenCASCADEConfig.cmake` ausentes |
| `--validate` sem OCCT | Falha explícita com saída 1, sem produzir relatório de sucesso |
| Abrir janela Wayland/X11 | Bloqueada: compositor e servidor X inacessíveis neste ambiente |
| Compilação e execução Windows | Pendente; workflow preparado e não executado |

Os testes cobrem volume analítico, validação de índices/triângulos, bordas abertas e orientação, zoom ancorado, STL ASCII/binário, escala, truncamento, contagens inválidas, valores não finitos, preservação de arquivos existentes, remoção de saídas incompletas e perda de detalhes na quantização f32.

## Evidência gráfica e consumo preliminar

Captura real produzida em `target/prototype/preview-solid.png`; relatório em `target/prototype/render-report.json`. Esses arquivos são artefatos locais ignorados pelo Git.

- Adaptador: llvmpipe, LLVM 23.1.3, renderização por software.
- Backend: OpenGL, através do wgpu.
- Captura: 960 × 640 pixels, com 41.314 pixels de superfície identificados.
- Tempo informado pelo comando: aproximadamente 224 ms, incluindo inicialização, renderização, leitura e gravação PNG.
- Pico RSS do processo no Linux: 169.508.864 bytes, aproximadamente 162 MiB, medido com `resource.getrusage` no processo que o iniciou.

Esses valores não representam fluidez interativa nem validam a meta de computador com 8 GB/GPU integrada. O teste usou uma malha pequena, build de desenvolvimento e renderização por software. A validação completa depende de SDK OCCT, Windows e hardware de referência.

## Pendências do ambiente no registro inicial (histórico)

- Sem resolução DNS pelo terminal: não foi possível obter o SDK OCCT ou atualizar dependências fora do cache.
- A pasta `.git` está protegida contra escrita e não contém um repositório inicializado. Não foi possível criar commits; nenhum remoto foi publicado.
- Consulte [PROTOTYPE.md](PROTOTYPE.md) para os requisitos e comandos reproduzíveis.
