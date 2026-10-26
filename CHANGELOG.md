# Histórico de mudanças

## Não lançado — 0.3.0 em desenvolvimento

### Modelagem e documento

- Rascunhos 2D com linhas, retângulos, círculos e arcos, geometria de construção, cotas de referência, aparar, estender, offset, espelhamento e refinamento de cantos; solver com indicação de graus de liberdade, conflitos e redundâncias.
- Parâmetros nomeados por peça com expressões e unidades, ligados às cotas dos rascunhos.
- Histórico paramétrico com extrusão, revolução, booleanas, arredondamento, chanfro, casca e transformação, construídos pelo OCCT fora da thread da interface.
- Montagem com subpeças, clones independentes e grupos; folhas técnicas associadas com vistas, cotas, corte e exportação PNG/PDF.

### Construção mecânica

- Furos posicionados por centros de círculo ou pontos isolados de um rascunho: cegos com ou sem ponta de broca, passantes, com rebaixo ou escareados, com medidas em expressões e opção de inverter a direção.
- Rosca simplificada registrada no furo (designação, diâmetro nominal e passo), sem hélice na geometria.
- Padrões linear (uma ou duas direções) e circular, espelhamento por plano com opção de manter o original e divisão de corpo por plano.
- Revolução em torno dos eixos do plano do rascunho ou de uma linha do rascunho, que acompanha as cotas.
- Padrões linear e circular de entidades no rascunho, com direção dada por uma linha ou centro dado por um círculo ou arco.
- Editor de operações reorganizado por ferramenta, com seções para cada operação.

### Montagem

- Padrão linear ou circular de instâncias, que compartilham a definição e ficam reunidas em um grupo; desagrupar libera cada posição.
- Espelhamento de instância como peça nova e independente: sólidos recebem uma operação final de espelhamento no histórico do clone, que continua paramétrico; corpos de malha são refletidos com a orientação das faces corrigida.
- Novo rascunho na face selecionada, de qualquer peça visível, com o plano capturado em coordenadas da peça de destino.

### Referências estáveis

- Faces de sólidos recebem nomes pela origem (entidade do rascunho, tampa, arredondamento, face herdada), registrados pelo histórico do OCCT na ponte C++.
- Regiões de extrusão/revolução são identificadas pelas entidades do contorno, e arestas/faces pelos nomes das faces. Mudar uma cota não troca mais a região usada nem quebra arredondamentos, chanfros e cascas posteriores; uma referência que deixa de existir gera erro pedindo nova seleção.
- Clonar uma peça remapeia os donos dos nomes de faces para os novos corpos.
- Formato `.3dnator` versão 7, com nomes de faces por corpo e migração explícita das referências por índice das versões anteriores.
- Seleção de região no editor de operações por lista com área e furos, em vez de índice numérico.
- Seleção gráfica de arestas e faces para arredondamento, chanfro e casca: o clique sobre uma face, ou perto de uma aresta, já gera a referência pelos nomes das faces. A malha dos sólidos guarda a face de origem de cada triângulo, também no arquivo.

### Visualização e desempenho

- Arestas vivas (dobras acima de 30°, bordas abertas) exibidas por padrão, calculadas uma vez por geometria.
- Triangulação e arestas desenhadas no shader por coordenadas baricêntricas, sem buffer de linhas: a memória da GPU por triângulo caiu de 216 para 84 bytes.
- Vértices de coordenadas idênticas são fundidos na importação STL e na tesselação, sem alterar a geometria. Uma malha fechada passa de 72 para cerca de 12 bytes por triângulo, também no arquivo de projeto.
- Comando `--benchmark` mede abertura, memória, navegação e recálculo, com cada cenário em processo próprio e GPU integrada por padrão. Na malha de 1 milhão de triângulos, o pico de memória caiu de 575 para 312 MiB e a abertura, de 507 para 330 ms.
- Ensaio `--validate-part` do critério de entrega da etapa 2 com o núcleo real.

### Núcleo CAD

- Corrigida a ordem dos componentes OCCT no CMake, que impedia a configuração com qualquer SDK 7.x.
- Ponte compilada e validada com OCCT 7.9.3 no Linux: os nove passos de `--validate` e os testes de nomeação com o núcleo real foram aprovados.
- Mensagens de estatística do OCCT não poluem mais o stdout dos relatórios JSON.
- A tesselação informa a face de origem de cada triângulo, base para seleção gráfica de faces e arestas.

## 0.2.0 — incremento anterior, sem tag

- Documento com peças, múltiplos corpos de malha, instâncias e transformações locais separadas da montagem.
- Comandos atômicos, desfazer/refazer por projeto e identificação de alterações desde o último salvamento; malhas compartilhadas entre snapshots.
- Pacote `.3dnator` versão interna 1, autocontido, com manifesto JSON e malhas f64; gravação temporária validada antes da substituição do arquivo principal.
- Abas independentes, árvore de peças/corpos, importação STL conforme contexto, renomeação, visibilidade, posicionamento numérico e exclusão de corpos.
- Diálogos nativos, Ctrl+S, exportação de corpo STL e confirmação de fechamento com Salvar/Descartar/Cancelar.
- Recuperação automática separada, com opção de restaurar cópias ao abrir o aplicativo e marcador que impede remover projetos normais na limpeza.
- Preparação de cena e buffers CPU em segundo plano, proteção contra resultados de outra aba/revisão e captura da interface real sem janela.
- Ensaio CLI do ciclo importar/salvar/reabrir sem o STL original, recuperação e exportação; CI estendida para esse fluxo.

## 0.1.0 — protótipo técnico anterior

Nenhuma versão foi publicada ou marcada com tag nesta sessão.

### Implementado no protótipo

- Workspace Rust com geometria própria, fronteira de núcleo CAD, intercâmbio, renderizador e aplicação experimental.
- Suporte em L de referência com volume e dimensões conhecidos.
- Leitura STL ASCII/binário com escala e limites de recursos; exportação binária com verificações de geometria e precisão, sem sobrescrita silenciosa.
- Visualizador egui/eframe/wgpu em português, com iluminação simples, câmera ortográfica, deslocamento, órbita, zoom no cursor e vistas predefinidas.
- Captura real do renderizador e nove testes de geometria, STL e navegação.
- Licença MIT para o código próprio, inventário inicial de dependências e documentação de execução e validação.

### Preparado para validação

- Ponte C++ opcional para OCCT 7.x, isolada por API Rust própria, com gestão de propriedade e conversão de exceções.
- Ensaios de sólidos, corte, chanfro, arredondamento, tesselação e ida e volta STEP/STL, com relatório de tempo e memória.
- Workflows de compilação/testes Rust e validação OCCT em Linux e Windows. Execução com SDK OCCT e execução Windows ainda pendentes.

### Planejado para o editor

- Projeto autocontido `.3dnator`, recuperação automática, abas, peças, corpos, instâncias, clones, grupos e montagem.
- Rascunhos com restrições, cotas, referências, diagnóstico, reparo, aparar, estender, offset e geometria de construção.
- Histórico paramétrico, parâmetros/expressões, operações atômicas e desfazer/refazer.
- Extrusão, revolução, booleanas, refinamentos, casca, furos, roscas simplificadas, padrões e espelhamento.
- Importação STEP como peças e STL como corpos de malha, referências independentes entre peças e exportação STL por corpo.
- Medição, cotagem, cortes, folhas técnicas associadas ao modelo e exportação PNG/PDF com opção blueprint.
- Linux e Windows como plataformas iniciais; referência de desempenho com 8 GB de RAM e GPU integrada.

Escopo e decisões completos em README.md, ARCHITECTURE.md e ROADMAP.md. Recursos planejados não são funcionalidades entregues neste protótipo.
