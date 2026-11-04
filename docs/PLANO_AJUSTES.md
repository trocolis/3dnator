# Plano dos ajustes pendentes

Análise de 04/11/2026 dos **11 itens desmarcados** de [AJUSTES.md](../AJUSTES.md).
Este documento planeja a implementação; nenhum item foi marcado como concluído.

A referência técnica é o código atual: aplicativo **0.32.0** em `Cargo.toml` e formato
**46** em `crates/project_io/src/project.rs`. Os números 0.9.0/formato 35 do guia de agentes
e partes do roteiro estão desatualizados. A análise foi estática, sem reproduzir os relatos
na interface nem executar os testes; causas ainda não demonstradas estão identificadas abaixo.

## Viabilidade e esforço relativo

Todos os itens são viáveis na arquitetura existente. Pequeno significa mudança localizada;
médio envolve interação, estado ou solver; grande atravessa geometria, histórico e persistência.
São estimativas de escopo, não prazos.

| Nº | Pendência | Esforço | Diagnóstico e direção |
| --- | --- | --- | --- |
| 1 | Texto do cubo no tema escuro | Pequeno | Faces têm fundo azul claro fixo; o texto usa `theme::ink()`, claro no tema escuro. Usar uma cor de texto própria com contraste sobre as faces. |
| 2 | Furo sobre rascunho não se modifica | Médio, sujeito à reprodução | A operação consulta os pontos atuais do rascunho, mas usa diâmetro próprio. Separar falha de posição/recálculo de expectativa de acompanhar o raio do círculo. |
| 3 | Navegação fecha a ferramenta | Médio | `Action::Select` chama `activate_selection`, que apaga vários painéis e prévias. Voltar da câmera à ferramenta deve apenas limpar `camera_mode`. |
| 4 | Remover ferramenta Rótulo | Pequeno na interface | Retirar os acessos de criação, mantendo leitura e recálculo de `Operation::Label` em projetos existentes. A infraestrutura de fontes também atende Texto. |
| 5 | Dimensionar até a origem | Médio | O seletor só reconhece pontos/curvas existentes. A origem do plano não é um alvo de cota e exige representação fixa e persistente. |
| 6 | Texto aberto até confirmar | Médio | O segundo clique chama diretamente `commit_sketch_text`. Transformá-lo em término do posicionamento, mantendo o painel. |
| 7 | Capturas e ângulo na direção do Texto | Médio | O gesto atual usa o cursor sem captura e calcula `atan2`. Integrar referências e o campo angular compartilhado com desenho. |
| 8 | Posição e prévia ao vivo do Texto | Médio a grande | O painel não oferece X/Y; o canvas mostra apenas ponto e guia. Reaproveitar contornos unitários e atualizar uma candidata fora do documento. |
| 9 | Furo em Criação e Casca em Modificação | Pequeno | A ordem atual em `shell::ribbon_groups` está invertida em relação ao pedido. |
| 10 | Ferramenta Pipe em Criação | Grande | Há guia e `MakePipeShell`, mas a Varredura atual exige dois perfis. Pipe precisa de contrato e operação próprios. |
| 11 | Converter peça para referência | Pequeno a médio | `Part::reference` e `Command::SetPartReference` já existem; falta a ação inversa no menu da peça e a transição segura da edição. |

Os números seguem a ordem das caixas pendentes no arquivo original. A mudança de posição
de Furo e Casca é um único item; os três pedidos de Texto serão implementados em conjunto.

## 1. Correções localizadas de apresentação

Entregar os itens 1, 4 e 9 antes das mudanças geométricas.

- **Cubo:** definir cor apropriada ao preenchimento das faces em `view_cube.rs`/`theme.rs`,
  incluindo hover. Conferir vistas frontais, isométrica e câmera rotacionada em ambos os temas;
  manter o critério existente que omite textos de faces pequenas ou muito inclinadas.
- **Rótulo:** remover botão, busca, atalhos e caminhos de criação em `shell.rs`,
  `feature_editor.rs` e módulos relacionados. Preservar `Operation::Label`, serialização,
  recálculo, exportação e recursos de fonte usados por `sketch_text`. Para operações antigas,
  conservar edição pelo histórico; remover esse acesso exigiria uma decisão adicional sobre
  compatibilidade. Texto no rascunho não substitui automaticamente relevo/gravação em face curva.
- **Faixa:** mover Furo para o grupo de criação e Casca para modificação, mantendo a mesma ação
  na busca. Conferir largura da faixa em 1280 pontos, inclusive após a inclusão de Pipe.

Aceite: cubo legível nos dois temas; nenhuma entrada para criar Rótulo; arquivo antigo com
Rótulo abre e recalcula; botões nos grupos solicitados e sem corte na faixa.

## 2. Preservação da ferramenta durante a navegação

Separar a intenção de **retomar a interação** da intenção de **encerrar a ferramenta**.

1. No botão Seleção da navegação, limpar apenas `shell.camera_mode` quando houver uma
   ferramenta suspensa por Arrastar/Rotacionar.
2. Manter o encerramento explícito ao cancelar, trocar de ferramenta e nos níveis existentes
   de Esc. Evitar mudar indiscriminadamente `activate_selection`, pois ele também prepara
   a abertura de outras ferramentas.
3. Conferir `selection_active`, realce do botão e roteamento dos eventos: gestos de câmera
   não devem adicionar alvos à pintura, cotas ou operações.
4. Conservar parâmetros, seleção transitória, fila de pintura e prévia. Resultados de outra
   aba/revisão continuam sendo descartados.

Arquivos principais: `shell.rs`, `selection_tools.rs`, `views.rs` e `paint_editor.rs`.
Aceite: iniciar Pintar, escolher cor/alvos, mover, rotacionar, voltar a Seleção e continuar;
repetir com Texto e uma operação com prévia, inclusive com cálculo em andamento.

## 3. Investigar e corrigir Furo

O relato não informa se mudou o centro, o raio do círculo, o rascunho selecionado ou um campo
da ferramenta. A causa permanece aberta. Hoje `hole_points` usa centros de círculos e pontos
isolados; `Operation::Hole` guarda seus IDs, e `features::rebuild` lê suas posições atuais.
O diâmetro é avaliado de `HoleSpec`, independentemente do raio dos círculos.

1. Criar regressões OCCT para mover o centro por cota/edição, editar o diâmetro no painel,
   alterar a profundidade e trocar o rascunho no painel. Medir posição e volume exatos.
2. Cobrir ponto removido, referências fixas vindas da face, texto no mesmo rascunho, corpo
   com posição local e edição no contexto de montagem. Verificar se a coleta de pontos inclui
   apoios que não deveriam posicionar furos.
3. Rastrear seleção da base, `kept_points`, direção, operação gerada, reconstrução e invalidação
   da prévia/cena. Corrigir a primeira divergência demonstrada.
4. Se o caso for mudança do raio do círculo, registrar que o comportamento atual usa diâmetro
   independente. Um vínculo opcional ao círculo é uma funcionalidade separada: exigiria
   referência estável à entidade, precedência clara sobre o diâmetro manual e persistência.
   Não mudar essa semântica silenciosamente para arquivos existentes.

Base de testes: `crates/commands/tests/mechanical_occt.rs` e `fdm_holes_occt.rs`.
O teste existente `counterbored_holes_follow_sketch_points_and_parameters` verifica espessura
e volume, mas não demonstra a atualização após mover um centro.

Aceite: alterações previstas atualizam B-rep e cena; referência perdida falha atomicamente;
salvar/reabrir e desfazer/refazer preservam o resultado. Se nenhuma regressão for reproduzida,
registrar exatamente os cenários cobertos e solicitar o caso original antes de declarar correção.

## 4. Dimensionar até a origem

Recomendação: representar a origem explicitamente como referência imutável do rascunho,
sem ID fictício e sem criar um ponto comum que possa ser movido ou virar alvo de Furo.

1. Acrescentar a escolha da origem em `dimension_tool::Pick` e definir sua resolução em
   `sketch::dimensions`. Ela corresponde a `[0, 0]` no plano local, inclusive em planos deslocados.
2. Definir restrições de distância/ΔX/ΔY até a origem e ponto–linha com origem. Compartilhar
   avaliação, resíduos, política de movimento, desenho e edição com as cotas existentes.
   Para círculos/arcos, a primeira entrega mede a distância do centro à origem; distância à
   borda deve ser apresentada como opção distinta se entrar no escopo.
3. Atualizar solver, parâmetros, validação, clonagem e persistência para os novos alvos/restrições;
   incrementar o formato somente ao implementar essa extensão. Manter leitura dos anteriores.
4. Priorizar captura da origem com tolerância de tela e sem confundi-la com a origem global
   da montagem. A origem permanece fixa ao editar o valor da cota.

Aceite: cota alinhada, horizontal e vertical de um ponto/centro à origem; distância de linha
à origem; edição de expressão, conflito, plano transformado, reabertura e desfazer/refazer.

## 5. Texto: uma entrega integrada

Implementar juntos os itens 6, 7 e 8, depois de estabilizar a navegação.

1. Criar etapas explícitas: inserir âncora, orientar e ajustar/confirmar. O segundo clique
   fixa a orientação; somente Confirmar publica a mudança. Cancelar descarta a candidata.
2. Acrescentar X/Y no plano local e carregar a posição real do apoio ao editar um texto existente.
   Conservar as unidades configuradas na interface e radianos/mm no modelo.
3. Reutilizar capturas de vértices, pontos médios, centros, arestas e referências de outros
   objetos, com a mesma política de profundidade e conversão ao plano usada pelo rascunho.
   Na primeira entrega, a direção capturada posiciona o texto; associatividade posterior
   a uma aresta não está implícita no pedido.
4. Integrar campo angular flutuante com expressões e conversão de unidades. Um ângulo digitado
   deve permanecer travado enquanto o cursor se move, conforme as ferramentas de desenho.
5. Calcular contornos em trabalhador e cachear por conteúdo/fonte/estilo. Posição, tamanho e
   rotação reaproveitam os contornos unitários; não consultar fontes a cada quadro.
6. Mostrar a candidata na posição final, substituindo visualmente o texto antigo na edição.
   Usar sequência da requisição, aba, rascunho e revisão para rejeitar respostas antigas.
   O retorno atual `SketchTextOutline` identifica aba/rascunho, mas precisa de proteção adicional
   para várias prévias concorrentes ou uma confirmação seguida de cancelamento.
7. Confirmar em uma única alteração atômica pelo fluxo de `UpdateSketch`/solver. Preservar IDs
   nas mudanças de transformação, como já faz `sketch::text`; erro conserva o painel e os valores.

Arquivos principais: `sketch_text.rs`, `project_editor.rs`, `sketch_editor/references.rs`,
infraestrutura de desenho/campos flutuantes, `commands::sketch_text` e `sketch::text`.
O modelo atual já guarda apoio, tamanho, rotação e contornos; esses campos de painel e prévia
não exigem, por si só, mudança de formato.

Aceite: criar e editar com prévia ao mudar cada propriedade; confirmar apenas pelo botão;
capturas e ângulo digitado; cancelar sem alterar documento; uma ação no desfazer; referências
de regiões preservadas quando só a transformação muda; respostas antigas nunca reaparecem.

## 6. Converter peça em objeto de referência

Adotar a semântica existente de `SetPartReference { reference: true }`: bloquear a edição e
mostrar como referência, conservando dados para voltar por **Criar peça**. Não exportar e
reimportar STEP para realizar a conversão, nem apagar histórico.

1. Adicionar a ação traduzida ao menu da peça em `project_tree.rs`/menu compartilhado.
2. Preparar a saída segura da edição. Com rascunho ou painel ainda não confirmado, desabilitar
   a conversão até confirmar/cancelar pelo fluxo existente, evitando perda de trabalho.
3. Após o comando, limpar contexto de peça ativa e prévias incompatíveis, selecionar a referência
   e respeitar a visibilidade da pasta Referências.
4. Explicitar que a marca pertence à definição: todas as instâncias da mesma peça tornam-se
   referências. Não clonar silenciosamente a ocorrência clicada.
5. Conferir subpeças: a árvore tem caminho próprio para referências de primeiro nível; testar
   componentes aninhados e impedir que a conversão deixe caminhos de edição internos ativos.

Aceite: corpo/B-rep, cores, posição e ocorrências preservados; mover/medir/ocultar continuam;
editar fica bloqueado; Criar peça, desfazer/refazer e salvar/reabrir funcionam. O formato 46
já persiste a marca. Cobrir a direção peça → referência, além do teste existente da inversa
em `crates/commands/tests/assembly.rs`.

## 7. Pipe

Premissa para tornar o plano concreto: **seção circular constante ao longo de uma trajetória**,
com diâmetro externo e modo maciço ou oco (espessura). É uma proposta de escopo, pois o item
original contém apenas o nome da ferramenta. Validar essa interpretação antes de implementar
o contrato persistente; perfil arbitrário e seção variável ampliam a proposta.

1. Fazer um ensaio técnico no kernel com linha e arco tangente, seção circular e anular,
   confirmando sólido fechado, volume e tratamento dos cantos. Reutilizar a infraestrutura
   nativa de `transitions.hpp`, sem forçar dois perfis artificiais no histórico da Varredura.
2. Primeira entrega: cadeia aberta e conectada de linhas/arcos de um rascunho, seção normal
   ao início da guia, novo corpo e dimensões por expressão. Rejeitar ramificações, curvas
   infinitas, espessura inválida e autointerseções com erro explícito. Não aceitar quinas cuja
   política de junção ainda não tenha sido validada pelo ensaio.
3. Introduzir especificação e `Operation::Pipe` em `document`, construção em `commands`, API
   própria em `kernel` e suporte sem OCCT com erro explícito. Compartilhar ordenação/validação
   de guia com `commands::sweep` sem carregar a exigência dos dois planos terminais.
4. Integrar dependências, normalização, remapeamento ao clonar, histórico, recálculo e nomes
   estáveis. Acrescentar persistência/migração e incrementar o formato a partir da versão
   vigente nesse momento, sem reservar um número antecipadamente.
5. Adicionar painel, seleção de cadeia, prévia no trabalhador, ícone e traduções em Criação.
   União/corte, caminhos fechados, arestas de sólidos e trajetórias espaciais ficam para
   extensão posterior, salvo confirmação de que fazem parte do pedido inicial.

Aceite: volumes analíticos de tubo reto e curvo, B-rep válido, falha atômica para caminho inválido,
edição de dimensões/guia com recálculo, desfazer/refazer, persistência e exportação STEP/STL.
A viabilidade básica é alta; limites de curvatura/cantos só podem ser afirmados após o ensaio.

## Ordem das entregas e validação

Sequência recomendada: **apresentação → navegação → Furo → cotas na origem → Texto →
conversão para referência → Pipe**. A conversão pode ser antecipada após a navegação; Pipe
é a maior entrega e não deve atrasar as correções. O diagnóstico de Furo deve ocorrer cedo.

Para cada alteração de código: fmt → clippy → testes, sempre com `--locked` nos comandos
Cargo que aceitam essa opção. Rodar os checks do workspace sem OCCT e, para geometria,
com OCCT e target separado, conforme `AGENTS.md`. Usar testes de comportamento para solver,
recálculo, atomicidade e persistência; alterações apenas visuais pedem capturas reais,
sem criar testes que apenas repitam a ordem dos botões.

Ampliar cenários de `--render-ui-scenario` para temas, navegação e Texto; usar diretórios novos.
Registrar o que as capturas não cobrem e o aceite manual pendente. Atualizar `CHANGELOG.md`
em cada entrega e só marcar a caixa correspondente em `AJUSTES.md` com evidências de aceite.
Não é necessário introduzir uma dependência nova para o plano proposto.

## Execução iniciada — 04/11/2026

- **0.32.1:** etapa 1 concluída (cubo, retirada dos acessos de criação de Rótulo e grupos Furo/Casca).
- **0.32.2:** etapa 2 concluída (navegação preserva a ferramenta e o comando de pintura).
- **0.33.0:** etapa 6 antecipada e concluída (converter peça em referência), com testes de persistência, instâncias compartilhadas e saída da edição na interface.
- Permanecem seis caixas pendentes: Furo, cotas na origem, os três ajustes de Texto e Pipe.
  A reprodução de Furo aguarda a descrição da alteração que não atualizou o resultado.

Evidências por versão em `docs/VALIDATION.md`. `AJUSTES.md` é ignorado pelo Git neste
repositório; suas caixas foram atualizadas localmente, sem forçar sua inclusão nos commits.

### Pipe — 0.34.0

Etapa 7 implementada com a seção circular constante proposta: Criação e busca, painel,
trajetória escolhida no viewport, prévia, novo corpo, edição pelo histórico, recálculo,
desfazer/refazer, clonagem, persistência no formato 47 e exportação STEP/STL. A primeira
entrega exige cadeia aberta com trechos tangentes e raio de curva maior que o raio externo.
Permanecem cinco caixas pendentes: Furo, cotas na origem e os três ajustes de Texto.
