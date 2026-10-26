# Formato de projeto — versão interna 8

A versão do pacote (`FORMAT_VERSION` em `crates/project_io/src/project.rs`) é independente da versão do aplicativo. O leitor aceita as versões 1 a 8; a gravação sempre usa a versão atual.

Um arquivo `.3dnator` é um ZIP com compressão Deflate e somente entradas declaradas:

```text
manifest.json
meshes/<body_id>.bin
breps/<body_id>.brep      (somente corpos sólidos CAD)
faces/<body_id>.bin       (somente corpos sólidos CAD, opcional)
```

## Manifesto

O manifesto registra `format = "3dnator"`, `schema_version`, versão do aplicativo que gravou, UUID do documento, nome, contador de identificadores, peças, instâncias, grupos e folhas técnicas. O campo opcional `recovery` distingue uma cópia automática de um projeto normal; quando ausente, vale `false`.

Cada peça contém:

- corpos: identificador, nome, visibilidade, transformação local, caminho da malha, caminho opcional do B-rep e, desde a versão 7, `face_names`;
- desde a versão 8, `color_srgb` opcional por corpo, com três canais inteiros de 0 a 255; ausência usa a aparência neutra;
- rascunhos: plano, pontos, entidades, restrições e expressões das cotas;
- parâmetros nomeados com expressões e dimensão (comprimento, ângulo ou escalar);
- histórico de operações (`features`), cada uma com nome, corpo resultante e operação.

Cada instância referencia uma definição de peça, a peça pai opcional (subconjuntos) e guarda posição/orientação relativas ao pai. A geometria de uma definição aparece uma vez no pacote, independentemente do número de instâncias.

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

Miniaturas e configurações persistentes de câmera ainda não fazem parte do esquema. A sequência de desfazer/refazer é somente da sessão e começa vazia ao reabrir um projeto.
