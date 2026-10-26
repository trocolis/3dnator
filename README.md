# 3dnator

Aplicativo CAD paramétrico em Rust para modelar peças mecânicas precisas para protótipos. O protótipo 0.3.0 já percorre o fluxo da primeira peça utilizável: rascunho cotado por parâmetros, extrusão, furos, arredondamentos, alteração de cota com recálculo, salvamento, reabertura e exportação STEP/STL, com o núcleo OCCT validado no Linux.

O projeto é de código aberto, sob a [licença MIT](LICENSE) para o código próprio. Dependências e recursos de terceiros mantêm suas respectivas licenças. Ainda não há repositório remoto publicado.

A interface organiza ferramentas por tarefa, com ícones vetoriais, busca pela tecla **S**, aba contextual de rascunho e painéis recolhíveis. Guia de uso e organização visual em [docs/INTERFACE.md](docs/INTERFACE.md). O refinamento no Linux é a prioridade atual; testes Windows estão adiados e disponíveis por execução manual da CI.

## Executar o protótipo

```sh
cargo run -p nator-prototype --features occt
```

Sem a feature `occt` (e o SDK OCCT 7.x instalado), o editor funciona com corpos de malha, mas não constrói sólidos. Consulte [docs/PROTOTYPE.md](docs/PROTOTYPE.md) para preparar o SDK, os comandos de linha e os ensaios, e [docs/PROJECT_FORMAT.md](docs/PROJECT_FORMAT.md) para o formato `.3dnator`.

O protótipo oferece, em resumo:

- projetos em abas, desfazer/refazer, salvamento autocontido e recuperação automática;
- rascunhos com restrições, cotas ligadas a parâmetros, aparar, estender, offset, espelhamento, padrões e refinamento de cantos;
- histórico paramétrico com extrusão, revolução, booleanas, arredondamento, chanfro, casca, furos (rebaixo, escareado, passantes, rosca registrada), padrões, espelhamento e divisão, com referências estáveis a regiões, faces e arestas;
- seleção de arestas e faces na área 3D, medição exata no B-rep e montagem com subpeças, clones, grupos, padrões e espelhamento de instâncias e conjuntos aninhados;
- folhas técnicas com vistas HLR e cortes do B-rep, cotas associativas reposicionáveis, múltiplos cortes hachurados e exportação PNG/PDF;
- exportação STL por corpo, por definição ou da montagem, com qualidade escolhida, e intercâmbio STEP de sólidos e montagens com cores e definições compartilhadas.

O que ainda falta do escopo abaixo está registrado em [ROADMAP.md](ROADMAP.md), na seção Situação atual. A [auditoria do plano](docs/PLAN_AUDIT.md) distingue implementação de aceite completo. Verificações locais e medições estão em [docs/VALIDATION.md](docs/VALIDATION.md).

As seções seguintes descrevem o escopo completo aprovado.

## Tecnologia e arquitetura

- Interface com egui, janela e eventos com eframe/winit e área gráfica com wgpu.
- Núcleo geométrico Open CASCADE (OCCT) 7.x, usado por uma ponte C++ própria e uma API Rust separada da interface. Validado no Linux com OCCT 7.9.3; pacotes locais verificados conforme [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md). A validação no Windows e a compatibilidade entre distribuições Linux continuam pendentes.
- Dependências C/C++ são permitidas para o núcleo geométrico, mantendo a aplicação em Rust. Faces dos sólidos recebem nomes pela origem, de modo que o histórico paramétrico referencia regiões, faces e arestas de forma estável.
- Português como idioma padrão, com estrutura para traduções.
- Linux e Windows são as plataformas iniciais; compilação, integração nativa e comportamento do editor devem ser validados em ambos. macOS não faz parte do escopo inicial.
- Máquina de referência inicial: computador com 8 GB de RAM e GPU integrada. É uma meta de validação, não um requisito mínimo comprovado ou garantia de desempenho; modelos concretos de CPU/GPU ainda serão escolhidos.
- Convenções de implementação em [AGENTS.md](AGENTS.md).
- Sequência de entregas aprovada em [ROADMAP.md](ROADMAP.md).
- Organização dos módulos e contratos em [ARCHITECTURE.md](ARCHITECTURE.md).

## Documento, peças e montagem

- Árvore hierárquica com peças, corpos, subconjuntos e grupos de organização.
- Construção restringe a edição a uma peça; montagem permite posicionar peças e usar outras como referência.
- Cada peça possui origem local; suas instâncias possuem transformações em relação à montagem ou ao conjunto que as contém.
- Instâncias compartilham uma definição; clones independentes permitem variações com histórico próprio.
- Referências entre peças capturam a geometria no momento de uso e não propagam mudanças posteriores. Dependências paramétricas ficam dentro da peça.
- Grupos permitem organizar e movimentar membros juntos sem criar vínculos fortes, nova peça ou origem persistente própria.
- STEP importado entra como peça; preservar cores sólidas quando disponíveis e permitir operações posteriores sobre a geometria importada.

## Modelagem

- Histórico paramétrico editável, com recálculo e indicação de falhas.
- Rascunhos em planos XY/XZ/YZ da peça ou faces planas de elementos visíveis. Faces externas geram referências independentes.
- Linhas, retângulos, círculos, arcos, cotas e restrições geométricas; vários contornos por rascunho.
- Atração para extremidades, pontos médios, vértices, arestas, centros e interseções; projeções no plano e guias pontilhadas de múltiplas referências.
- Restrições automáticas no próprio rascunho; referências externas não criam dependências entre peças.
- Extrusão de regiões com distância para um lado, simétrica ou duas distâncias; novo corpo, união, corte ou interseção. Limite até uma face respeita a independência entre peças.
- Revolução por eixo ou linha, com volta completa ou ângulo definido.
- União e divisão de corpos.
- Chanfro e arredondamento em rascunhos e corpos, com prévia e medidas editáveis; raio constante e chanfros por distâncias ou distância/ângulo. Propagação por arestas tangentes disponível.

## Posicionamento e navegação

- Mover peças ou corpos livremente, numericamente ou por alinhamento face a face, aresta a aresta e vértice a vértice, sem relação permanente.
- Pivô padrão no centro de massa; vértice alternativo como pivô ou aresta reta como eixo de rotação. Grupos usam o centro de massa combinado.
- Botão esquerdo para selecionar/desenhar; scroll para zoom no cursor; botão do meio e arraste para deslocar a vista; Shift com botão do meio e arraste para orbitar.
- Projeção ortográfica padrão e perspectiva opcional.
- Cubo de orientação com faces, arestas e cantos clicáveis, sincronizado com a câmera, e indicação de X/Y/Z.

## Desfazer e refazer

- Ctrl+Z desfaz; Ctrl+Shift+Z ou Ctrl+Y refaz. A sequência pertence ao projeto e inclui alterações de construção e montagem.
- Cada ação concluída constitui um passo; um arraste completo ou movimento de grupo constitui uma única ação.
- Prévias não entram na sequência; Esc cancela a operação em andamento. Seleção e navegação da câmera ficam fora dela.
- Salvar preserva a sequência durante a sessão; reabrir o projeto inicia uma nova sequência na versão inicial.
- O histórico paramétrico é persistido no arquivo e é distinto da sequência de desfazer/refazer.
- Excluir uma operação com dependentes exige mostrar os itens afetados e oferecer cancelar ou excluir também os dependentes, com prévia.

## Edição do histórico paramétrico

- Duplo clique abre os parâmetros de uma operação; permitir renomear e suprimir operações preservando seus parâmetros.
- Permitir voltar a uma etapa para visualizar o resultado naquele ponto e inserir novas operações.
- Permitir reordenar operações apenas quando suas dependências forem respeitadas.
- Se uma edição causar falha, mantê-la pendente e indicar a primeira operação problemática e suas dependentes. Oferecer corrigir a referência, suprimir a operação afetada ou cancelar a edição.
- Manter o último resultado válido disponível para comparação, claramente identificado como anterior à edição pendente.
- Nunca substituir silenciosamente uma referência perdida por outra face ou aresta.

## Medição, cotagem e desenho técnico

- Ferramenta Medir informa distância entre vértices e componentes X/Y/Z, comprimento de arestas, raio/diâmetro e comprimento de arcos, ângulo e distância entre faces paralelas, menor distância entre corpos/peças, área, volume e centro de massa.
- Medições não alteram a geometria. Coordenadas seguem a origem da peça na construção e a origem da montagem nesse contexto.
- Milímetros são a unidade padrão; campos aceitam unidades explícitas como mm, cm e in. Casas decimais de exibição são configuráveis e não definem a precisão interna.
- Sem materiais definidos, o centro de massa assume densidade uniforme; essa hipótese deve ser indicada nas propriedades.
- Ferramentas de cotagem desenham medidas lineares, angulares, raios e diâmetros, com linhas auxiliares, setas e texto reposicionável. Distinguir cotas que controlam o rascunho das anotações de medição e documentação.
- Corte de visualização por plano deslocável, com inversão do lado visível, preserva os corpos. Nas folhas, vistas em corte possuem identificação, como A–A, e hachuras nas regiões seccionadas.
- Espaço de desenho técnico com folhas A4/A3, orientação e escala configuráveis; vistas frontal, superior, lateral, isométrica e cortes; legenda com nome da peça, revisão, data e unidade.
- Exportar PNG com resolução configurável e PDF vetorial. Oferecer aparência técnica convencional e estilo blueprint com fundo azul e linhas claras.
- Folhas são salvas no projeto e associadas à peça ou montagem escolhida. Vistas e cotas acompanham alterações do modelo; referências perdidas são sinalizadas para revisão, sem reassociação silenciosa.

## Parâmetros e expressões

- Incluir parâmetros nomeados por peça desde a primeira versão, editáveis em tabela, com valores e expressões.
- Os nomes definidos pelo usuário podem estar em português; identificadores do código do aplicativo continuam em inglês.
- Cotas e campos dimensionais das operações aceitam valores diretos ou expressões, por exemplo `largura - 20 mm`.
- Recalcular dependentes dentro da peça, sem criar vínculos paramétricos entre peças diferentes.
- Validar compatibilidade de unidades e dependências, rejeitando ciclos e expressões inválidas com indicação do erro e preservação do último modelo válido.

## Estado dos rascunhos

- Indicar estados livre/parcialmente restrito, totalmente restrito e em conflito, além de sinalizar restrições redundantes.
- Usar cores e símbolos para comunicar os estados e permitir destacar os elementos envolvidos em conflitos ou redundâncias.
- Arrastar entidades parcialmente restritas respeita as restrições existentes, permitindo apenas os movimentos restantes.
- Permitir extrusão de regiões válidas e fechadas de rascunhos parcialmente restritos; não exigir que todas as dimensões e posições estejam determinadas.

## Refinamento dos rascunhos

- Aparar trechos de entidades até suas interseções com outras geometrias.
- Estender linhas e arcos até outra geometria e criar contornos paralelos por deslocamento (offset) com distância definida.
- Permitir alternar entidades entre geometria normal e de construção. Geometria de construção usa traço interrompido, participa de cotas/restrições e pode servir de eixo de revolução ou simetria, mas não delimita regiões de extrusão.
- Permitir cotas de referência, que exibem medidas calculadas sem controlar a geometria. Ao alternar o tipo de cota, validar os efeitos sobre o sistema de restrições.
- Realçar regiões fechadas sob o cursor para seleção de extrusão e destacar pontas abertas, inclusive pequenos espaços entre segmentos.
- Diagnosticar entidades duplicadas ou sobrepostas e trechos de comprimento zero. Permitir múltiplos contornos e furos internos; nos cruzamentos, mostrar as regiões resultantes e indicar quais são utilizáveis.
- Quando uma extrusão falhar por problema no perfil, destacar a geometria envolvida e explicar a correção necessária.
- Não fechar espaços nem remover entidades silenciosamente. A ferramenta Reparar rascunho sugere correções, apresenta prévia e exige confirmação antes de aplicá-las.

## Furos e roscas

- Ferramenta de furos com diâmetro e profundidade, opção passante, rebaixo cilíndrico e escareado com diâmetro e ângulo.
- Posicionamento por pontos de rascunho, incluindo vários furos na mesma operação, com seleção dos corpos afetados da peça ativa.
- Operações editáveis no histórico, com suporte a parâmetros e expressões nas medidas.
- Roscas inicialmente simplificadas: armazenar diâmetro nominal, passo e profundidade, com indicação no desenho técnico. A representação simplificada não inclui hélice na geometria exportada.
- Geometria real de rosca fica para uma etapa posterior.

## Padrões e espelhamento

- Padrão linear em uma ou duas direções, definido por quantidade e espaçamento; padrão circular por eixo, quantidade e ângulo total.
- Espelhamento em relação a uma linha no rascunho ou a um plano no espaço.
- Disponibilizar repetição de entidades no rascunho, de operações ou corpos na construção e de instâncias na montagem.
- Na construção, padrões pertencem ao histórico e acompanham alterações dos elementos originais.
- Na montagem, as instâncias compartilham a definição da peça. Permitir desvincular uma posição do padrão para movimentação individual ou clonar a definição para criar uma variação independente.
- Espelhamento de montagem que exija geometria invertida cria uma nova peça independente, como uma versão esquerda ou direita.

## Casca

- Transformar um sólido em peça oca com espessura definida por valor ou expressão.
- Permitir remover uma ou várias faces, ou manter todas para produzir uma cavidade fechada.
- Construir paredes para dentro ou para fora do sólido original.
- Oferecer prévia e indicar falhas quando a espessura não for geometricamente possível.
- Registrar a operação no histórico paramétrico, com parâmetros editáveis.

## Interface e visualização

### Seleção

- Na montagem, o clique seleciona uma instância; na construção, faces, arestas, vértices e corpos da peça ativa; no rascunho, entidades, pontos, cotas e restrições.
- Ferramentas filtram os candidatos conforme os tipos de geometria aceitos.
- Ctrl com clique adiciona ou remove elementos da seleção; clique no vazio limpa a seleção.
- O alvo sob o cursor é realçado antes da seleção. Árvore e área 3D mantêm a seleção sincronizada.
- Retângulo da esquerda para a direita seleciona elementos inteiramente contidos; da direita para a esquerda seleciona os elementos tocados.
- Segurar o botão esquerdo abre os candidatos sobrepostos, com destaque na área 3D ao percorrer a lista.

### Disposição e aparência

- Mostrar/ocultar peças, corpos e grupos pela árvore; a visibilidade de uma instância não altera a das demais instâncias da mesma peça.
- Isolar a seleção temporariamente e restaurar a visibilidade anterior ao sair do isolamento.
- Permitir transparência e bloqueio de seleção, mantendo os elementos visíveis.
- Durante a construção, esmaecer as demais peças e mantê-las disponíveis como referências explícitas das ferramentas.
- Visibilidade, isolamento, transparência e bloqueio de seleção são controles visuais: não alteram geometria nem suprimem operações paramétricas.
- Abrir vários projetos em abas na mesma janela. Cada projeto mantém câmera, seleção e sequência de desfazer/refazer próprias; painéis e ferramentas exibem o projeto ativo.
- Indicar alterações não salvas nas abas. Ao fechar um projeto modificado, oferecer Salvar, Descartar ou Cancelar.
- Permitir importar peças de outra aba como cópias independentes, sem criar vínculos entre os arquivos.
- Ferramentas no topo, árvore à esquerda, propriedades à direita e histórico da peça ativa embaixo; painéis recolhíveis.
- Ferramentas com prévia, confirmação e cancelamento; Esc cancela a operação atual.
- Cor neutra padrão, cores sólidas editáveis e iluminação simples com arestas visíveis.
- Sombras opcionais e desligadas por padrão; transparência sob demanda. Texturas e reflexos elaborados ficam para avaliação posterior.
- Redesenho sob demanda, reaproveitamento de malhas entre instâncias e separação entre malha de exibição e geometria precisa.

## Salvamento e intercâmbio

- O salvamento padrão usa um formato próprio de projeto, preservando peças, instâncias, montagem, grupos, rascunhos, restrições, histórico paramétrico e configurações do documento.
- O projeto é autocontido: um único arquivo inclui as definições de peças, montagens e a geometria importada de STEP, sem depender dos arquivos de origem. Instâncias compartilham a definição interna da peça.
- Importar peças de outro projeto cria cópias independentes, preservando o compartilhamento entre suas instâncias dentro do projeto de destino. Vínculos com arquivos externos ficam fora do escopo inicial.
- Salvamento manual por Ctrl+S e salvamento automático de recuperação, para restaurar trabalho após falhas.
- Peças e montagens podem ser exportadas para STEP. A exportação não substitui o projeto com seu histórico de edição.
- Incluir exportação STL para prototipação em impressoras 3D na primeira entrega utilizável. A unidade padrão de exportação é o corpo: gerar um STL por corpo selecionado, inclusive quando uma peça contém vários corpos. A seleção de uma peça permite exportar seus corpos separadamente.
- Nomear arquivos separados com peça e corpo, resolvendo colisões sem sobrescrever silenciosamente arquivos existentes.
- Oferecer exportação conjunta em um único STL, preservando as posições relativas dos elementos, sem realizar união booleana automática. Corpos unidos por uma operação booleana são exportados como o corpo resultante.
- Exportar STL binário por padrão, com coordenadas em milímetros, opções de qualidade rascunho/padrão/alta e tolerância personalizada; exibir dimensões, quantidade de arquivos e estimativa de tamanho antes de exportar.
- Para sólidos CAD, gerar a malha de exportação da geometria precisa, independentemente da malha de exibição. Validar fechamento, orientação de faces e triângulos degenerados.
- Arquivos separados usam coordenadas locais da peça; exportação conjunta usa posições de montagem.
- Exportar cada corpo de uma definição compartilhada uma única vez por padrão, informando a quantidade de ocorrências. Oferecer exportação de todas as instâncias nas posições de montagem.
- Importar STL como corpo de malha pertencente a uma peça: na montagem, criar uma nova peça; na construção, inserir o corpo na peça ativa. A peça pode conter também corpos sólidos e subpeças.
- Preservar a malha importada dentro do arquivo `.3dnator`, sem depender do STL de origem. Identificar claramente corpos de malha e corpos sólidos CAD na árvore.
- Inicialmente, permitir visualizar, mover, rotacionar, medir e usar vértices e triângulos da malha como referências, respeitando sua precisão. Não inferir círculos ou superfícies curvas exatas automaticamente.
- Operações entre malhas e sólidos exigem suporte específico e ficam fora do escopo inicial. Conversão de malha em sólido é uma ferramenta futura e explícita, sem conversão automática na importação.
- Na importação STL, mostrar dimensões e unidade assumida e permitir corrigir a escala antes de confirmar.
- A extensão do projeto é `.3dnator`. O arquivo funciona como um pacote com estrutura do documento, rascunhos, restrições, parâmetros, histórico, geometria dos corpos e importações, folhas técnicas, cores, configurações do documento e miniatura.
- O formato possui versão interna independente da versão do aplicativo e deve prever migrações de projetos antigos. A codificação e o mecanismo de empacotamento ainda serão definidos.
- O salvamento escreve primeiro em arquivo temporário e substitui o projeto anterior somente após concluir e validar a gravação. A recuperação automática fica separada do arquivo salvo manualmente.

## Versionamento do projeto

Versões X.Y.Z e commits descritivos conforme [AGENTS.md](AGENTS.md). As decisões acima descrevem o escopo desejado, não uma lista de recursos já entregues.
