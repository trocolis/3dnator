# Formato de projeto — versão interna 1

Implementado no protótipo 0.2.0. A versão do pacote é independente da versão do aplicativo.

Um arquivo `.3dnator` é um ZIP com compressão Deflate e entradas declaradas:

```text
manifest.json
meshes/<body_id>.bin
```

O manifesto registra `format = "3dnator"`, `schema_version = 1`, versão do aplicativo que gravou, UUID do documento, nome, contador de identificadores, definições de peças e suas instâncias. O campo opcional `recovery` distingue uma cópia automática de um projeto normal; quando ausente, vale `false`.

Cada peça contém corpos. Cada corpo possui identificador, nome, visibilidade, transformação local e caminho de sua malha. Cada instância referencia uma definição de peça e guarda posição/orientação relativas à montagem. A geometria de uma definição aparece uma vez no pacote, independentemente do número de instâncias.

Transformações usam deslocamentos em milímetros e rotações em radianos, aplicando X, depois Y, depois Z. A transformação do corpo é aplicada antes da transformação da instância. Movimentar a montagem não altera os vértices locais.

Cada malha usa:

| Campo | Codificação |
| --- | --- |
| Assinatura | 8 bytes ASCII `3DMSH001` |
| Quantidade de vértices | u32 little-endian |
| Quantidade de triângulos | u32 little-endian |
| Vértices | Três f64 little-endian por vértice |
| Triângulos | Três índices u32 little-endian por triângulo |

A precisão do projeto não é reduzida a f32, como ocorre na exportação STL binária. Malhas continuam sendo malhas; esse formato não reconstrói superfícies curvas exatas nem histórico CAD.

## Integridade e limites

- Até 256 MiB de arquivo, 512 MiB descompactados e 8 MiB de manifesto.
- Até dez mil elementos, dois milhões de triângulos e seis milhões de vértices, com limites também na cena resultante das instâncias visíveis.
- Validação de números finitos, índices, triângulos, nomes, identificadores únicos e referências entre peças/instâncias.
- Caminhos de malha são derivados dos identificadores. O leitor não extrai entradas para disco nem segue caminhos externos.
- Versões futuras e campos desconhecidos são rejeitados para evitar descarte silencioso de dados. Ainda não há migração, pois esta é a primeira versão do formato.

O salvamento cria um arquivo temporário no diretório de destino, conclui e sincroniza a gravação, reabre e valida o pacote e só então substitui o arquivo principal. Falhas anteriores à substituição preservam o arquivo anterior. Isso não constitui garantia contra toda falha de hardware ou sistema de arquivos.

## Recuperação

O aplicativo agenda uma cópia de recuperação dois segundos após uma edição, quando a thread de trabalho estiver disponível. O arquivo principal não é sobrescrito pela recuperação. Cada sessão/aba usa um nome próprio; ao iniciar, o aplicativo oferece as cópias remanescentes.

Os diretórios padrão são `$XDG_STATE_HOME/3dnator/recovery` ou `$HOME/.local/state/3dnator/recovery` no Linux, e `%LOCALAPPDATA%/3dnator/recovery` no Windows. `NATOR_RECOVERY_DIR` permite escolher outro diretório.

Uma cópia recuperada abre sem caminho de salvamento principal e com alterações pendentes. Salvar ou descartar explicitamente o projeto remove suas cópias conhecidas. Uma falha na recuperação é informada e não altera o documento em memória.

A descoberta e a limpeza verificam o marcador `recovery = true` no manifesto. Arquivos normais na mesma pasta são preservados e não são substituídos pelo gravador de recuperação. A busca considera até mil arquivos recentes e oferece até cinquenta cópias identificadas.

## Escopo ainda pendente

Miniaturas, configurações persistentes de câmera, sólidos CAD, rascunhos, parâmetros, histórico paramétrico, grupos e subconjuntos ainda não fazem parte deste esquema. A sequência de desfazer/refazer é somente da sessão e começa vazia ao reabrir um projeto.
