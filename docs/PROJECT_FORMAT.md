# Formato de projeto — versão interna 27

A versão do pacote (`FORMAT_VERSION` em `crates/project_io/src/project.rs`) é independente da versão do aplicativo. O leitor aceita as versões 1 a 27; a gravação sempre usa a versão atual.

Um arquivo `.3dnator` é um ZIP com compressão Deflate e somente entradas declaradas:

```text
manifest.json
meshes/<body_id>.bin
breps/<body_id>.brep      (somente corpos sólidos CAD)
faces/<body_id>.bin       (somente corpos sólidos CAD, opcional)
images/<image_id>.rgba    (imagens de referência, opcional)
appearances/<body_id>.mesh       (cache de pintura, opcional)
appearances/<body_id>.materials  (um byte por triângulo do cache)
```

## Manifesto

Desde a versão 27, `appearances` opcional guarda uma entrada por corpo pintado: `body`, `layers`, `source_signature`, `mesh_path` e `materials_path`. Cada camada contém uma `FaceRef`, uma cor sRGB `[r, g, b]` e `mask` opcional com `sketch`/`region` (`RegionRef`, inclusive `Multiple`). O rascunho deve pertencer à peça do corpo. A camada posterior prevalece dentro de sua área. O limite é 64 camadas por corpo; o rascunho usado não pode ser excluído enquanto houver pintura dependente.

A malha de aparência usa o mesmo cabeçalho/payload de `meshes`, em coordenadas locais do corpo, sem substituir o B-rep ou a malha original. O arquivo de materiais contém exatamente um byte por triângulo: zero herda a cor do corpo/ocorrência, e N usa a cor da camada N−1. Os dois caminhos devem estar presentes juntos ou ser nulos. Caminhos são derivados do ID e o leitor confere assinatura do B-rep, contagem de materiais, índices e limites geométricos. Assim o projeto reabre com as cores mesmo sem OCCT; recalcular a delimitação exige esse núcleo. Copiar/clonar remapeia corpos, faces nomeadas e rascunhos; os caches são compartilhados em memória. STEP/STL continuam exportando a geometria original; os recortes de aparência e cores por face não são exportados.

Desde a versão 26, `name_counters` opcional no manifesto guarda a última sequência usada por tipo (`Part`, `Body`, `Sketch`, `WorkPlane`, `Image`). São contadores de criação independentes dos IDs globais e dos nomes editáveis. Renomear ou excluir não os reduz; desfazer/refazer restaura o estado correspondente. Na leitura de versões antigas, a sequência inicial usa a quantidade existente e os sufixos padrão ainda presentes. Nomes próprios e nomes importados são preservados.

`images` opcional no manifesto contém `id`, `sketch`, `name`, `width`, `height`, `pixel_path`, `origin`, `size`, `transparency` e `visible`. `origin` é o canto inferior esquerdo, e `size` é largura/altura positivas, em mm no plano local do rascunho indicado. O plano/posicionamento desse rascunho fornece as coordenadas na peça/montagem. `transparency` varia de 0 a 1; 0,5 corresponde aos 50% iniciais da interface. O payload `images/<id>.rgba` contém exatamente `width × height × 4` bytes RGBA de 8 bits, com linhas de cima para baixo, comprimido pelo ZIP. Não há caminho externo nem cabeçalho de imagem a interpretar. O leitor confere caminho derivado do ID, existência do rascunho, dimensões, quantidade de pixels e limites: 256 imagens, até 8192 pixels por lado, 64 MiB por imagem e 128 MiB de pixels no documento. IDs pertencem ao espaço global. Excluir um rascunho remove suas imagens; copiar uma peça remapeia os rascunhos/IDs das imagens e compartilha os pixels em memória.

Extrusão aceita `RegionRef::Multiple`, serializado como `{"Multiple": [referência, ...]}`, com referências do mesmo rascunho. A lista é plana, sem repetições, de uma a 1024 áreas. Cada referência mantém a representação `Area`, `Boundary` ou a compatibilidade antiga `Index`; novas edições normalizam os índices. O recálculo une os prismas, e os nomes das faces recebem uma chave derivada do contorno, independente da ordem da seleção. Revolução continua exigindo uma única região.

Desde a versão 25, `dimension_targets` opcional no rascunho associa o ID de uma cota aos dois elementos na ordem em que foram escolhidos: `Point(id)` ou `Entity(id)`, serializados como objetos com esse nome e ID local. A chave deve identificar uma cota existente e os dois alvos devem existir no mesmo rascunho. O campo é omitido quando vazio. Editar o valor ou mover o texto preserva esses alvos, o ID da cota e `dimension_labels`.

A política de resolução prefere preservar o elemento com mais conexões dirigentes; conta vizinhos distintos ligados por apoios compartilhados/restrições ativas e apoios de cotas/fixações, ignorando ligações fracas e a própria cota em edição. Em empate, prefere preservar o segundo escolhido. Essa preferência é transitória: não grava travas nem altera as restrições existentes. Sem o campo, a ordem é inferida dos campos da restrição quando possível; não é possível recuperar a ordem original de cliques de projetos anteriores. Cotas de uma única curva não recebem um par artificial.

Modos de Espelhar e Deslocamento continuam sendo intenções de criação, gravadas como geometria comum. Espelhar conserva apoios compartilhados e coincidências internas nas cópias. Normal usa distância assinada; Dupla usa duas distâncias positivas, uma por lado; Simétrica usa a mesma distância positiva em cada lado. Em caminhos conectados, o lado positivo é a esquerda da orientação ordenada do caminho; linhas e arcos são recortados nos encontros dos suportes deslocados. Em uma linha isolada, vale o sentido início→fim; em círculos/arcos isolados, o valor positivo aumenta o raio. Não há novo vínculo associativo com a origem da cópia.

Mover, seleção conectada, prévias e atalhos não acrescentam campos ao manifesto. O nome de interface Referência conserva a representação `construction`; retas infinitas continuam exigindo esse sinalizador.

Desde a versão 24, `PointOnCurve { point, curve }` liga um apoio à circunferência de um círculo/arco, acompanhando seu centro e raio no recálculo. `PointOnLine` continua sendo a incidência à reta suporte. Ligações geométricas `Coincident`, `Midpoint`, `PointOnLine` e `PointOnCurve` com `reference = true` são fracas: seus alvos são preservados, mas não acrescentam resíduos ao solver. Com `false`, são fortes. A força não equivale a uma restrição `Fixed` de posição absoluta.

Abrir um ponto compartilhado por entidades cria cópias coincidentes e ligações fracas entre os apoios. Cruzamentos podem ter um ponto auxiliar com incidências às curvas, sem dividir as entidades; esse apoio é preservado pela limpeza de pontos enquanto houver suporte. Ao liberar um apoio auxiliar sem cotas ou outras relações próprias, ele é removido e o cruzamento volta a ser virtual. Não há campo novo nos pontos para a força: ela é derivada dos apoios compartilhados e das restrições. O formato anterior já permite as ligações geométricas existentes; nenhum projeto antigo exige reconstrução dos contornos.

Desde a versão 23, pontos e entidades de rascunho aceitam `projected`, omitido quando falso. Projetar captura coordenadas no plano local do rascunho, sem vínculo posterior com o componente de origem. Os apoios projetados e raios dos círculos projetados são constantes do solver e recebem também restrições explícitas `Fixed`/`Radius`. A marca distingue o traço lilás e o preenchimento claro das regiões fechadas. A geometria continua disponível para captura, dimensionamento e perfis. Círculos/arcos paralelos ao plano são analíticos; curvas inclinadas e outras curvas usam polilinhas com desvio máximo de 0,01 mm. Projetos anteriores abrem sem marcas de projeção.

O manifesto registra `format = "3dnator"`, `schema_version`, versão do aplicativo que gravou, UUID do documento, nome, contador de identificadores, peças, instâncias, grupos e folhas técnicas. O campo opcional `recovery` distingue uma cópia automática de um projeto normal; quando ausente, vale `false`.

Cada peça contém:

- corpos: identificador, nome, visibilidade, transformação local, caminho da malha, caminho opcional do B-rep e, desde a versão 7, `face_names`;
- desde a versão 8, `color_srgb` opcional por corpo, com três canais inteiros de 0 a 255; ausência usa a aparência neutra;
- rascunhos: plano fixo, pontos, entidades, restrições, expressões das cotas e visibilidade (usada pelo viewport e pelo olho da árvore);
- desde a versão 20, `work_planes` opcional: planos de trabalho com identificador, nome, visibilidade e `plane` (`base`, plano com origem e eixos U/V unitários e perpendiculares em mm, e `offset`, deslocamento em mm ao longo de U × V, limitado a ±10.000.000 mm). O plano resultante é calculado na leitura. Omitido quando vazio;
- desde a versão 21, `support` opcional em cada rascunho de peça: identificador de um plano de trabalho da mesma peça. O plano do rascunho continua gravado e é o que o solver e as operações usam; editar o plano de trabalho reescreve os planos dos rascunhos apoiados. Ausente, o plano do rascunho é fixo; rascunhos de referência nunca têm o campo;
- parâmetros nomeados com expressões e dimensão (comprimento, ângulo ou escalar);
- histórico de operações (`features`), cada uma com nome, corpo resultante, operação e, desde a versão 12, `suppressed` opcional (operação suprimida, com parâmetros preservados);
- desde a versão 12, `history_marker` opcional: quantidade de operações construídas a partir do início. As posteriores ficam inativas, com a última geometria oculta, e novas operações entram nessa posição. Ausente, todo o histórico é construído; o marcador nunca aponta para o fim (forma canônica).

Desde a versão 19, `Tangent { line, arc, point }` também aceita um círculo completo em `arc`: `point` deve ser o identificador do centro, e a distância entre o centro e a reta suporte deve ser igual ao raio. Arcos conservam a regra anterior de extremidade compartilhada. A mudança de versão impede que leitores antigos interpretem esse vínculo como tangência de arco. Não é necessária transformação dos projetos anteriores.

Os novos modos de construção são gravados como entidades e restrições comuns: duas linhas colineares de comprimentos iguais para a linha simétrica, quatro lados ortogonais para o retângulo e centro/raio para círculos. O modo e a cadeia de cliques são estado temporário da interface. Dimensões digitadas mantêm suas expressões; no círculo por dois pontos, a expressão de raio é metade do diâmetro informado.

Desde a versão 18, entidades de rascunho aceitam `infinite` opcional, omitido quando `false`. Somente `Curve::Line` com `construction = true` pode usar `infinite = true`. Seus pontos `start`/`end` parametrizam posição e direção para o solver, sem limitar a reta nem fornecer capturas de extremos/ponto médio. Cópias rígidas e espelhamentos preservam o tipo; áreas/extrusões ignoram a entidade. Projetos anteriores mantêm segmentos finitos pelo valor padrão.

Desde a versão 22, rascunhos aceitam as cotas `Diameter { circle, value }` (círculo ou arco), `PointLineDistance { point, line, value }` (distância sem sinal do ponto à reta suporte; o ponto não pode ser extremo da linha) e `LineCircleDistance { line, circle, value, crossing }` (da reta suporte à borda, perpendicular à linha; `crossing`, omitido quando falso, mede quanto a borda passa da reta). Valores em mm, com expressões de comprimento. `LineAngle` aceita `flip_line` e `flip_reference`, omitidos quando falsos: o lado oposto (fim→início) de cada linha, para que o valor seja exatamente o ângulo do setor cotado. `dimension_labels` opcional associa o identificador de uma cota à posição do texto, em mm no plano e relativa à âncora (ponto médio do que se mede, centro do círculo ou vértice do ângulo). Ausente, o texto fica no lugar padrão.

A restrição `LineAngle { line, reference, value }` mede a direção de `line` em relação à direção de `reference`, em radianos e no sentido anti-horário. As duas entidades devem ser linhas distintas existentes. Expressões são avaliadas como ângulo, incluindo números em graus quando não há unidade explícita. A cota acompanha a referência no recálculo, e referências angulares atualizam seu valor medido. `Angle { a, b, value }` continua medindo uma direção em relação ao eixo X local.

Desde a versão 14, o manifesto pode conter `reference_sketches`: rascunhos de referência da montagem, com o mesmo formato dos rascunhos de peça e coordenadas da montagem. Os identificadores compartilham o espaço dos rascunhos de peça. Não pertencem a peça nenhuma, não têm parâmetros (cotas guardam valores numéricos) e nenhuma operação pode usá-los como perfil; servem apenas de destino para posicionar componentes. Ausente, a lista é vazia.

Desde a versão 15, o manifesto e cada peça podem conter `folders`, a visibilidade das pastas da árvore. No projeto: `project`, `parts`, `sketches` (rascunhos de referência) e `origin`, todos `true` por padrão. `project` é mantido por compatibilidade e normalizado para `true` na leitura: a raiz atual é sempre visível. Na peça: `bodies`, `subparts`, `sketches`, `planes` (desde a versão 20; `true` por padrão) e `origin` (`false`). A visibilidade efetiva de um corpo, rascunho ou instância combina a dele com a das pastas acima; a de cada elemento é preservada. O campo só é gravado quando difere do padrão.

Cada instância referencia uma definição de peça, a peça pai opcional (subconjuntos) e guarda posição/orientação relativas ao pai. A geometria de uma definição aparece uma vez no pacote, independentemente do número de instâncias.

Desde a versão 9, instâncias também aceitam `color_srgb` opcional. A cor da ocorrência mais específica no caminho de montagem prevalece sobre a cor do corpo, permitindo importar cores distintas em instâncias de uma definição compartilhada. Ausência conserva a aparência da definição.

Transformações usam deslocamentos em milímetros e rotações em radianos, aplicando X, depois Y, depois Z. A transformação do corpo é aplicada antes da transformação da instância. Movimentar a montagem não altera os vértices locais.

## Malhas e B-rep

Cada malha usa:

| Campo | Codificação |
| --- | --- |
| Assinatura | 8 bytes ASCII `3DMSH001` |
| Quantidade de vértices | u32 little-endian |
| Quantidade de triângulos | u32 little-endian |
| Vértices | Três f64 little-endian por vértice |
| Triângulos | Três índices u32 little-endian por triângulo |

A precisão do projeto não é reduzida a f32, como ocorre na exportação STL binária. Vértices de coordenadas idênticas são compartilhados. A malha de um corpo sólido é a malha de exibição; a geometria precisa está no B-rep, gravado no formato texto do OCCT (`BRepTools::Write`, até 64 MiB por corpo).

Para sólidos, `faces/<body_id>.bin` (campo `faces_path`) guarda a face do B-rep de origem de cada triângulo da malha, o que permite selecionar faces e arestas na área 3D:

| Campo | Codificação |
| --- | --- |
| Assinatura | 8 bytes ASCII `3DFAC001` |
| Quantidade | u32 little-endian, igual ao número de triângulos da malha |
| Faces | Um u32 little-endian por triângulo, menor que a quantidade de nomes de faces |

Sem essa entrada, o sólido continua editável pelo histórico, mas a seleção gráfica pede que a operação seja recalculada.

## Nomes de faces e referências estáveis (versão 7)

`face_names` tem um nome por face do B-rep, na ordem topológica do núcleo (`TopExp::MapShapes`), que a serialização B-rep preserva. Os nomes descrevem a origem da face:

| Origem | Nome |
| --- | --- |
| Lateral de extrusão/revolução | `b{corpo}:e{entidade do rascunho}` |
| Tampas da extrusão/revolução | `b{corpo}:start`, `b{corpo}:end` |
| Face nova de arredondamento/chanfro | `b{corpo}:round`, `b{corpo}:chamfer` |
| Borda e face interna de casca | `b{corpo}:rim`, `{nome original}:inner` |
| Face preservada ou recortada | nome da face de entrada |
| Sólido STEP importado | `b{corpo}:f{índice}` |

Nome vazio indica face sem origem conhecida, que não pode ser referenciada.

As operações referenciam a geometria pela origem, não pela posição:

- `RegionRef::Boundary(entidades)`: região de rascunho identificada pelas entidades do contorno externo;
- `EdgeRef::Between([face, face])`: aresta entre duas faces nomeadas;
- `FaceRef::Named(face)`: face nomeada.

As variantes `Index` (`RegionRef::Index`, `EdgeRef::Index`, `FaceRef::Index`) guardam a referência posicional das versões anteriores. Elas mantêm o significado antigo: a região é a posição na lista ordenada por área, e a aresta/face só vale enquanto a assinatura do B-rep de entrada for idêntica. A próxima edição da operação as converte em referências estáveis quando as faces envolvidas têm nome.

Quando a origem de uma referência desaparece ou fica ambígua, o recálculo falha com uma mensagem pedindo nova seleção. Nenhuma outra região, aresta ou face é escolhida no lugar.

## Migrações

- 26 → 27: pinturas opcionais por face, com máscaras de rascunho e caches de aparência. Ausência significa aparência uniforme anterior; não há recálculo na abertura.

- 25 → 26: sequências opcionais de nomes e imagens incorporadas; ausências significam sequências inferidas e nenhuma imagem. Extrusões podem guardar múltiplas referências de áreas, sem alterar a interpretação das seleções antigas.
- 17 → 18: `Entity::infinite` opcional e restrição `LineAngle`. Ausência do campo mantém todas as linhas antigas finitas; não há conversão de geometria nem alteração dos perfis existentes.
- 16 → 17: `Operation::ExtrudeFace` registra corpo/face de origem, limites, combinação opcional e face limite opcional. A origem é referência mesmo quando Novo corpo não a consome. `RegionRef::Area` registra um ciclo canônico de pares (entidade, sentido invertido), distinguindo células que usam as mesmas curvas. Regiões antigas `Boundary`/`Index` preservam sua interpretação anterior; não há conversão implícita que altere sólidos existentes. O modo automático só é resolvido na criação do comando; o arquivo guarda Unir/Remover/Interseção ou Novo corpo concretos. Laterais de extrusão de face recebem nomes derivados dos papéis das faces adjacentes à aresta de origem, sem depender da ordem das arestas.
- 15 → 16: rascunhos podem conter a restrição `Angle { a, b, value }`, orientação do vetor entre dois pontos em relação ao eixo X local. `value` usa radianos; expressões associadas são avaliadas como ângulo (números sem unidade usam graus). Referências angulares medem a geometria sem dirigi-la. Não há campos obrigatórios novos nos projetos anteriores.
- 14 → 15: o manifesto e as peças acrescentam `folders` (ausente vale o padrão: tudo visível, exceto a origem das peças). A origem global, antes um controle da sessão sempre visível ao abrir, passa a ser gravada.
- 13 → 14: o manifesto acrescenta `reference_sketches` (ausente vale lista vazia). A gravação omite o campo quando não há rascunhos de referência.
- 12 → 13: o manifesto acrescenta `settings` (configurações do documento, hoje `display_decimals`, de 0 a 6; ausente vale 2) e o pacote pode conter `thumbnail.png`, uma miniatura isométrica de até 4 MiB conferida apenas pela assinatura PNG. Projetos anteriores abrem com as configurações padrão e sem miniatura.
- 11 → 12: operações acrescentam `suppressed` (ausente vale `false`) e peças acrescentam `history_marker` (ausente vale `None`). Projetos anteriores abrem com todo o histórico ativo. Resultados de operações inativas (suprimidas, dependentes de uma suprimida ou posteriores ao marcador) precisam estar ocultos; o leitor rejeita o contrário.
- 10 → 11: folhas técnicas acrescentam cotas associativas (`annotations`) com referências estáveis de faces e arestas (`DrawingTopologyRef`) e caminhos de ocorrência em montagens. Projetos anteriores desserializam com lista de cotas vazia.
- 9 → 10: folhas acrescentam `orientation` (padrão `Landscape`), `png_dpi` (150) e `additional_sections` (vazio). O campo `section` continua representando A–A; cortes adicionais guardam identificadores de 1 a 25 e o plano (origem em mm, normal), sem renumeração após remoções. São aceitos até sete cortes adicionais, com identificadores únicos, normais válidas e PNG de 72 a 300 DPI.
- 8 → 9: `color_srgb` ausente em uma instância assume `None`; geometria, cores de corpos e compartilhamento são preservados.
- 7 → 8: `color_srgb` ausente assume `None`, preservando a aparência neutra dos projetos anteriores. A cor não altera malhas, B-rep ou referências.
- Versões 1 a 6: campos acrescentados ao longo dessas versões (rascunhos, parâmetros, histórico, B-rep, subconjuntos, grupos, folhas) são opcionais e assumem valores vazios quando ausentes.
- 6 → 7 (`migrate_to_7`): antes da desserialização tipada, a região numérica de extrusões/revoluções vira `{"Index": n}`, e o par `edge`/`face` + `signature` de arredondamento, chanfro e casca vira `{"Index": {"index": n, "signature": s}}`. Um histórico com campos ausentes é rejeitado inteiro, sem migração parcial.

## Integridade e limites

- Até 256 MiB de arquivo, 512 MiB descompactados e 8 MiB de manifesto.
- Até dez mil elementos, dois milhões de triângulos e seis milhões de vértices, com limites também na cena resultante das instâncias visíveis.
- Validação de números finitos, índices, triângulos, nomes, identificadores únicos e referências entre peças, instâncias, rascunhos e operações.
- `face_names` só é aceito em corpos com B-rep; cada nome tem até 256 bytes e nenhum caractere NUL.
- Caminhos de malha e B-rep são derivados dos identificadores. O leitor não extrai entradas para disco nem segue caminhos externos.
- Versões futuras e campos desconhecidos são rejeitados para evitar descarte silencioso de dados.

O salvamento cria um arquivo temporário no diretório de destino, conclui e sincroniza a gravação, reabre e valida o pacote e só então substitui o arquivo principal. Falhas anteriores à substituição preservam o arquivo anterior. Isso não constitui garantia contra toda falha de hardware ou sistema de arquivos.

## Recuperação

O aplicativo agenda uma cópia de recuperação dois segundos após uma edição, quando a thread de trabalho estiver disponível. O arquivo principal não é sobrescrito pela recuperação. Cada sessão/aba usa um nome próprio; ao iniciar, o aplicativo oferece as cópias remanescentes.

Os diretórios padrão são `$XDG_STATE_HOME/3dnator/recovery` ou `$HOME/.local/state/3dnator/recovery` no Linux, e `%LOCALAPPDATA%/3dnator/recovery` no Windows. `NATOR_RECOVERY_DIR` permite escolher outro diretório.

Uma cópia recuperada abre sem caminho de salvamento principal e com alterações pendentes. Salvar ou descartar explicitamente o projeto remove suas cópias conhecidas. Uma falha na recuperação é informada e não altera o documento em memória.

A descoberta e a limpeza verificam o marcador `recovery = true` no manifesto. Arquivos normais na mesma pasta são preservados e não são substituídos pelo gravador de recuperação. A busca considera até mil arquivos recentes e oferece até cinquenta cópias identificadas.

## Escopo ainda pendente

A miniatura (`thumbnail.png`, 256 × 192, vista isométrica sombreada da cena visível) é gerada a cada gravação na CPU, sem GPU; projetos sem corpos visíveis não têm miniatura. Configurações persistentes de câmera ainda não fazem parte do esquema. A sequência de desfazer/refazer é somente da sessão e começa vazia ao reabrir um projeto.
