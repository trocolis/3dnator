# Arquitetura do aplicativo

Estrutura aprovada para um workspace Rust. O incremento 0.3.0 implementa `geometry`, `sketch`, `kernel`, `document`, `commands`, `project_io`, `renderer` e `drawing` (folhas técnicas a partir de projeções próprias ou malhas). A interface ainda vive em `apps/prototype/src/desktop`, que reúne os papéis planejados de `ui` e `app`; a extração para crates próprios fica para quando a interface estabilizar.

## Módulos

| Módulo | Responsabilidade |
| --- | --- |
| `geometry` | Coordenadas, unidades, transformações, identificadores e tipos geométricos compartilhados |
| `sketch` | Entidades 2D, restrições, solução e identificação de regiões |
| `kernel` | Integração com OCCT, sólidos, operações e tesselação, através de API própria |
| `document` | Peças, corpos, instâncias, grupos, parâmetros e histórico paramétrico |
| `commands` | Ações de edição, transações, desfazer/refazer, coordenação do recálculo e consultas que usam o núcleo (medição, planejamento de exportação) |
| `project_io` | Formato `.3dnator`, migrações, recuperação e importação/exportação |
| `renderer` | Visualização em wgpu, seleção gráfica e destaques |
| `drawing` | Folhas técnicas: vistas, cortes, cotas, notas e exportação PNG/PDF |
| `ui` | Painéis, ferramentas interativas, atalhos e traduções |
| `app` | Inicialização e integração com Linux e Windows |

A câmera de apresentação oferece projeção ortográfica (padrão) e perspectiva com campo vertical de 45°. A escala é definida no plano do alvo para manter pan, zoom e movimento livre consistentes entre projeções. Matriz da GPU, raios de seleção e projeção das medições usam a mesma câmera, mantida por aba durante a sessão; pontos recortados em profundidade não recebem anotações.

## Limites e dependências

- A interface solicita alterações por comandos, em vez de modificar diretamente o documento.
- O espelhamento de montagem clona a hierarquia e seus grupos internos. Cada definição descendente é refletida uma única vez no plano local X = 0; posições de ocorrências compensam essa reflexão com duas inversões, preservando transformações rígidas e o compartilhamento entre componentes repetidos. A raiz conserva sua posição e usa o plano escolhido convertido para suas coordenadas. Sólidos ganham operações finais no histórico; malhas são refletidas com a orientação corrigida. O comando inteiro é atômico.
- As regras geométricas e de negócio não dependem da interface ou do renderizador. Geometria, histórico e persistência devem poder ser verificados sem abrir uma janela.
- `geometry` permanece independente dos módulos superiores. `sketch` e `kernel` não dependem de `document`, `commands`, `ui` ou `app`.
- `document` usa os tipos e APIs próprios de geometria, rascunhos e núcleo; não recebe tipos públicos de OCCT.
- `sketch::drafting` resolve a prévia das primitivas com medidas livres/travadas no plano local (mm/radianos), sem estado de interface. A interface coleta expressões e cria a geometria cotada apenas ao confirmar; `document::parameters` avalia cotas lineares e angulares pela dimensão correspondente, e o solver roda no trabalhador. A derivada angular desembrulha o corte de ±π para preservar a orientação ao recalcular.
- `commands` coordena as alterações do documento e o recálculo. `project_io` usa o documento e as APIs internas para codificar, decodificar e converter dados, sem depender da interface.
- `renderer` recebe dados de apresentação derivados do documento, sem se tornar a fonte da geometria precisa.
- `drawing` recebe projeções vetoriais e curvas de corte próprias ou malhas e a especificação da folha; não depende do núcleo. `commands::drawing` coordena HLR e interseções de planos com o B-rep para contextos exclusivamente sólidos e o método aproximado para malhas/contextos mistos. Cortes preservam grupos por corpo para não tratar sobreposições de peças como furos de material. Cotas associativas apoiam-se em referências estáveis de faces e arestas e caminhos de ocorrência, extraindo centroides, centros circulares e orientações no B-rep; referências perdidas tornam-se avisos na folha sem quebrar a regeneração. O reposicionamento das cotas altera apenas a apresentação e preserva a geometria calculada. O documento guarda a especificação e fornece notas derivadas do histórico.
- `ui` apresenta o estado, conduz interações e solicita comandos e operações de arquivo; `app` conecta esses serviços ao ciclo de vida da aplicação.
- A definição dos crates concretos e de seus contratos deve respeitar esses limites, sem dependências circulares.

## Referências topológicas estáveis

O histórico paramétrico nunca guarda posições em listas que mudam com as cotas:

- Regiões de rascunho são identificadas pelas entidades do contorno externo (`RegionRef::Boundary`). Se o contorno for refinado, a região continua reconhecida enquanto metade das entidades originais existir e todas estiverem em um único contorno.
- Cada face de um sólido recebe um nome pela origem, calculado a partir do histórico do OCCT (`Generated`/`Modified`) na ponte C++: entidade do rascunho que gerou a lateral, tampas, faces novas de arredondamento, chanfro, casca ou furo, e faces herdadas de booleanas, transformações e padrões. Os nomes têm a forma `b{corpo}:...` e são gravados com o B-rep.
- Arestas são referenciadas pelo par de faces adjacentes (`EdgeRef::Between`) e faces pelo nome (`FaceRef::Named`). A interface pode selecionar por índice ou por clique; o comando converte a escolha em referência estável antes de gravá-la.
- Uma referência cuja origem desapareceu, ou ficou ambígua, faz o recálculo falhar com mensagem pedindo nova seleção. Nenhuma outra geometria é escolhida no lugar.

A lógica de nomes fica em `kernel/src/naming.rs` (Rust puro, testado sem o SDK); a ponte só informa a origem de cada face.

## Operações, prévias e erros

- No incremento atual, o documento usa identificadores tipados e geometria de malha imutável compartilhada por Arc. Transformações de corpos são locais à peça; instâncias aplicam uma transformação adicional na montagem.
- `Session` publica uma edição somente após validar sua cópia candidata; desfazer/refazer mantém até 100 snapshots leves. A revisão da sessão cresce também ao desfazer/refazer, enquanto o identificador do estado permite reconhecer o último salvamento.
- Cenas e buffers CPU são preparados na thread de trabalho. O resultado de importação identifica a aba e a revisão originais, e uma cena antiga não substitui a revisão atual. A interface mantém apenas os buffers GPU da aba ativa.
- O formato de projeto (versão 11) serializa peças, corpos de malha e sólidos (B-rep, nomes de faces e face de origem de cada triângulo), rascunhos, parâmetros, histórico paramétrico, montagem, grupos e folhas técnicas com cotas associativas. O recálculo de cada operação ocorre na preparação do comando, fora da thread da interface, sobre uma cópia candidata do documento.

- Uma operação confirmada é atômica: sua conclusão atualiza o documento e adiciona uma ação à sequência de desfazer, ou a falha preserva o documento anterior.
- Prévias e edições pendentes usam estado separado até a confirmação. Uma edição paramétrica com falha permanece pendente para correção, supressão da etapa afetada ou cancelamento, conforme as regras do editor.
- Um arraste completo e uma operação sobre vários elementos podem compor uma única transação de edição.
- Cálculos demorados ficam fora da thread da interface; resultados devem ser associados à revisão que os originou, impedindo que um resultado antigo sobrescreva uma edição mais recente.
- Erros da integração nativa devem ser convertidos em erros da API própria do núcleo. Gestão de memória e detalhes de interoperabilidade ficam isolados nessa integração.

## Próxima validação

OCCT está integrado e validado no Linux. Falta a mesma validação no Windows (job `occt-windows` da CI) e no Ubuntu com OCCT 7.6, além da avaliação de distribuição das bibliotecas compartilhadas.

## Intercâmbio STEP

A ponte XCAF isola documentos, definições e ocorrências do OCCT atrás de `StepAssembly`. A camada de comandos converte a importação em definições independentes, com B-reps, malhas e nomes posicionais de faces; a aplicação publica o comando preparado apenas na revisão capturada. Corpos e ocorrências guardam cores sRGB, com a ocorrência mais específica sobrepondo a aparência da definição. Matrizes de intercâmbio passam por validação de ortonormalidade e determinante antes de se tornarem `Placement`. Leitores e escritores STEP compartilham uma exclusão mútua porque o intercâmbio OCCT usa configurações globais; a espera fica na thread de trabalho.

## Interface e apresentação

O shell desktop mantém tema, ícones vetoriais e organização de ferramentas em módulos próprios de `apps/prototype/src/desktop/project_editor/` (`theme`, `icons` e `shell`). Menu, faixa e busca encaminham as mesmas ações aos editores e comandos existentes; o estado dos painéis pertence à interface. Nenhuma regra geométrica depende desses módulos. A cor de fundo é configurável no renderizador e participa do esmaecimento visual das referências, sem modificar a geometria ou a cor dos corpos no documento. Uso e responsabilidades em [docs/INTERFACE.md](docs/INTERFACE.md).

O navegador `view_cube` é um widget próprio: projeta um poliedro fechado, seleciona seus polígonos e limita o desenho à sua área. A câmera em `renderer` mantém a direção e a inclinação da tela em radianos; órbita, projeção, seleção, pan e zoom usam a mesma base ortonormal. Botões de vistas e faces do cubo escolhem a mesma orientação convencional. O estado é da câmera de cada aba, sem alteração do documento ou do formato de projeto.

### Rascunhos no viewport CAD

O editor usa uma única câmera para sólidos e rascunhos. A camada de apresentação projeta pontos locais pelo plano do rascunho e pela posição hierárquica da ocorrência; o cursor percorre o caminho inverso, intersectando o raio da câmera com o plano. A base U/V determina o alinhamento e a orientação da tela. Transições interpolam uma base rígida, alvo e escala; novos limites da cena reenquadram tanto a câmera atual quanto os extremos da transição. Sem sólidos, os limites vêm dos rascunhos visíveis.

A seleção de face consulta o B-rep exato pelo módulo de medições dos comandos, em segundo plano e com revisão identificada. A interface converte o suporte ao referencial da peça e publica `AddSketch`. O plano é uma captura fixa, sem referência associativa à face. `SetSketchVisibility` registra a escolha no documento; criar uma operação que usa rascunho oculta o perfil na mesma transação. Reconstruir o histórico não reescreve essa preferência. A origem de apresentação e a escolha de suporte ainda não confirmada pertencem à aba, sem modificar o documento.
