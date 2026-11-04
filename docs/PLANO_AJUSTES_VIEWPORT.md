# Plano dos ajustes de precisão, cubo e seção

Análise de 04/11/2026 das **quatro caixas pendentes** de [AJUSTES.md](../AJUSTES.md)
(o arquivo citado como `AJUSTE.md` está no repositório com esse nome, no plural).
Base verificada: aplicativo **0.37.0**, formato **49**. O
[plano anterior](PLANO_AJUSTES.md) permanece como histórico das entregas anteriores.

Esta análise é estática: foram conferidos os caminhos de configuração, câmera,
renderização, seleção e persistência. Não houve reprodução gráfica nem execução
de testes. As duas imagens citadas no arquivo não foram encontradas entre os
arquivos disponíveis; o diagnóstico dos glitches precisa de reprodução antes
de ser tratado como causa confirmada. Nenhuma caixa foi marcada como concluída.

## Viabilidade e prioridade

Todos os pedidos são viáveis. Os esforços abaixo representam escopo relativo,
não prazo ou garantia de implementação antes dos ensaios indicados.

| Item | Viabilidade | Esforço | Direção |
| --- | --- | --- | --- |
| Precisão única, padrão 0,1 mm | Alta | Pequeno | Manter `mouse_step_mm`, retirar presets e tornar o campo numérico permanente. |
| Cubo conserva o lado predominantemente acima | Alta | Médio | Escolher a orientação ortogonal mais próxima da base atual ao clicar numa face. |
| Seção separa corpos encostados sem glitch | Alta, com ensaio de renderização | Grande | Preparar tampas reais de visualização e contornos por corpo/ocorrência, com profundidade. |
| Hover não mostra componentes ocultos na seção | Alta | Médio a grande | Recortar os realces no plano e testar sua profundidade contra a cena e suas tampas. |

Premissa para o primeiro item: “uma precisão” significa **um valor linear
configurável**, sem alternativas predefinidas. A precisão angular tem outra unidade
e continua independente; casas decimais, tolerâncias do OCCT e tesselação também
mantêm seus contratos. Essa interpretação corresponde ao controle atual e ao valor
pedido em milímetros. Não transformar 0,1 mm em tolerância do núcleo.

## 1. Precisão linear em campo único

**Evidência.** `Settings` já persiste apenas `mouse_step_mm` em
`crates/document/src/lib.rs`. O padrão é `DEFAULT_MOUSE_STEP_MM = 0.4`.
`settings_drawing`, em `settings.rs`, oferece presets e Personalizado e muda o passo
ao trocar de unidade quando reconhece um preset. O rascunho também usa esse valor
na fusão de novas pontas; cada ponto conserva sua precisão de criação.

Implementação:

1. Alterar o padrão para **0,1 mm**, mantendo os limites atuais de 0,001 a 1000 mm.
2. Em `settings_drawing`, substituir presets/Personalizado por um campo sempre
   visível, reaproveitando `command_panel::length_value`, confirmação da digitação
   e `Command::SetSettings` pelo fluxo existente. Remover estado temporário e APIs
   de presets que ficarem sem uso.
3. Ao trocar mm/polegada, conservar o valor físico em mm e converter sua exibição.
   Não substituir 0,1 mm por uma fração em polegadas.
4. Conservar o valor explícito dos projetos existentes, inclusive 0,4 mm. Projetos
   antigos sem o campo recebem o novo padrão pelo `serde(default)` existente.
   Não modificar a precisão guardada em pontos já criados.
5. Atualizar textos necessários em `i18n`, documentação e expectativas dos testes.

**Aceite:** documento novo começa em 0,1 mm; digitação com vírgula funciona;
valores inválidos são recusados; troca de unidade preserva a medida; desfazer/refazer
e salvar/reabrir conservam o valor. Cobrir desenho, captura e fusão com o novo padrão,
pois a mudança afeta geometria nova, além da aparência da configuração.

Base: testes de configurações em `crates/document/src/lib.rs`, compatibilidade em
`crates/project_io/src/project.rs` e ensaios de precisão em `capture.rs`.
Não é necessária mudança de formato.

## 2. Orientação do cubo preserva o alto da tela

**Evidência.** O clique em `view_cube.rs` chama `Camera::set_direction`.
Esse método, em `crates/renderer/src/lib.rs`, redefine `roll = 0` e impõe um azimute
nos polos. Isso explica a perda da orientação anterior no caminho do clique.
`Camera` já possui base completa e `set_orientation`, capazes de conservar o referencial.

Implementação:

1. Criar um método de câmera específico para alinhar pelo cubo. Preservar o contrato
   de `set_direction`, usado também para vistas canônicas e Home.
2. Para a normal da face clicada, gerar quatro bases ortonormais separadas por 90°
   em torno dela. Escolher a que mais se aproxima da orientação atual, priorizando
   o produto escalar entre o vetor vertical candidato e o vertical atual.
3. Quando a projeção do vertical antigo no novo plano for quase nula, usar o vetor
   horizontal antigo para desempatar. Definir ordem canônica estável para empates
   exatos, sem oscilações por arredondamento. Não arredondar apenas o `roll` antigo:
   sua referência muda quando muda a direção de observação.
4. Limitar a nova escolha aos cliques nas seis faces principais na primeira entrega;
   conservar o comportamento das arestas/cantos, das setas e de Home. Se a mesma
   regra for estendida a diagonais, definir antes a base canônica correspondente.
5. Conservar alvo, enquadramento e projeção. Um novo clique na mesma face deve
   manter a orientação escolhida.

**Aceite:** testar as seis faces após órbita e roll, inclusive vista invertida,
polos e transições próximas de 45°. O eixo que predominava acima permanece acima
quando sua projeção permite; o resultado final está alinhado em passos de 90°.
Conferir coerência de projeção, picking e arraste nas projeções ortográfica e
perspectiva, além de capturas reais do cubo.

## 3. Hover respeita corte e oclusão

**Evidência.** `section::first_visible` já descarta interseções no lado removido
e é usado por hover e clique. Porém, `pick_overlay` pode coletar o contorno de um
componente inteiro, inclusive seus outros corpos. `highlight::draw` projeta esse
contorno via egui sem receber o plano de seção nem consultar o depth buffer.
Seu teste de normal não resolve oclusão. `hover_context` também não inclui a seção
ou a câmera, permitindo reuso temporário após mudanças nesses estados.

Implementação em duas partes:

1. Acrescentar seção e estado da câmera à invalidação do hover; limpar resultados
   incompatíveis quando plano, lado, revisão, aba ou visibilidade mudarem. Aplicar
   o mesmo contrato às consultas assíncronas relacionadas.
2. Recortar segmentos, triângulos, contornos e pontos em coordenadas de mundo pelo
   semiespaço visível, interpolando segmentos que cruzam o plano. Não basta remover
   primitivas cujo primeiro ponto está cortado.
3. Renderizar o hover 3D em passe com teste de profundidade no viewport, compatível
   com o corte e a projeção. Manter rótulos e mira como elementos 2D. Usar pequeno
   bias apenas para evitar conflito com a própria face, sem atravessar outros corpos.
4. Preservar separadamente os contratos dos realces selecionados e das prévias de
   movimento. A correção de hover não deve esconder inadvertidamente os manipuladores
   que precisam continuar acessíveis.
5. Conferir coerência com clique, lista de sobrepostos, seleção de face/aresta/vértice,
   pintura, referência de plano e capturas. Diferenciar o que foi removido pelo corte
   do que permanece no modelo, mas está atrás de material visível.

**Aceite:** um corpo totalmente removido não ganha hover; o contorno de um corpo
parcialmente cortado termina no plano; componentes atrás de material opaco não
aparecem através dele. Cobrir conjuntos aninhados, isolamento, bloqueio, alterações
do plano com cursor parado e as duas projeções. Fixar o comportamento de transparência
explicitamente no teste, respeitando as opções de seleção já existentes.

Esta entrega pode corrigir primeiro recorte e cache. O aceite completo de oclusão
na seção depende das tampas com profundidade da próxima etapa.

## 4. Tampas e divisórias entre corpos na seção

**Evidência.** `view.wgsl` descarta o lado removido e pinta os fragmentos de costas
como hachura. Não há tampa geométrica no plano. A cena usa `LessEqual` e não conserva
identidade de corpo nos atributos enviados ao shader. Faces coincidentes podem
disputar profundidade; essa hipótese combina com o relato, mas ainda exige reprodução.
A hachura atual também não desenha contornos próprios de cada seção.

Ensaio inicial obrigatório:

- Montar dois blocos encostados, duas ocorrências da mesma peça, um corpo com furo
  e uma malha aberta; comparar vistas normais e oblíquas, invertendo o corte.
- Registrar capturas antes da mudança e identificar se o artefato envolve superfícies
  laterais coincidentes, hachura, ou ambos. Medir o custo de atualizar o plano durante
  arraste e a quantidade de segmentos gerada em cenas representativas.

Implementação recomendada após o ensaio:

1. Preparar a seção por **corpo e caminho da ocorrência**, antes de perder essa
   identidade na malha agregada. Considerar posição local e transformações aninhadas;
   respeitar os estilos e a visibilidade usados por `prepare_scene_with_removal`.
2. Para o viewport, calcular interseções da malha de apresentação com o plano e
   montar laços por corpo, com tolerância numérica própria e estável. Há rotinas de
   segmentos/laços em `nator_drawing` que merecem avaliação; não copiar cegamente seu
   contrato nem criar dependência do renderer com document/commands.
3. Triangular regiões fechadas preservando ilhas e furos. Trata-se de geometria de
   visualização: não substituir B-rep, medição ou exportação por esse resultado.
   Caso o ensaio demonstre necessidade de curvas precisas, o kernel já oferece
   `Solid::section_segments`; sua adoção deve ocorrer no trabalhador para sólidos
   reais, sem simular OCCT quando indisponível.
4. Desenhar tampas no plano e contornos com teste de profundidade. Desativar a falsa
   tampa por faces traseiras onde existir a nova tampa. Manter tratamento explícito
   para malhas abertas, sem inventar fechamentos para laços incompletos.
5. Conservar contornos de corpos diferentes, mesmo quando compartilham uma borda.
   Deduplicar apenas o traço coincidente para evitar espessura dobrada, mantendo sua
   identidade lógica. Não unir os corpos para produzir a seção. Hachuras com fases
   ou direções distintas podem ajudar, mas a linha divisória é o requisito principal.
6. Se houver fragmentos coplanares coincidentes, adotar regra determinística de
   composição no passe da seção. Apenas aumentar epsilon ou deslocar cada corpo
   não resolve a ausência de divisória nem garante estabilidade.
7. Executar interseção/laços/triangulação fora da thread da interface. Cachear por
   geometria, ocorrência, plano e estado visual; identificar requisições por aba,
   revisão e geração. Durante o arraste, limitar e substituir pedidos pendentes;
   somente a geração atual pode publicar a tampa. Nunca mostrar tampa antiga sob
   um plano novo. A câmera só precisa invalidar o cálculo se a estratégia escolhida
   depender dela.
8. Fazer hover e clique respeitarem as tampas opacas. A consulta atual à malha
   original pode alcançar faces atrás da tampa: avaliar interseção do raio com a
   região tampada antes de aceitar tais alvos. Na primeira entrega, a tampa é um
   obstáculo visual, sem criar uma face CAD selecionável artificial.

**Aceite:** linha contínua entre blocos encostados, sem cintilação ao orbitar ou
inverter o corte; furos e cavidades continuam vazios; mesma peça em várias ocorrências
não mistura contornos; corpos ocultos não geram tampas. Cobrir contato parcial,
plano sobre face/aresta/vértice, escalas distintas, transparência e malhas abertas.
Desabilitar ou cancelar a seção restaura a cena original, sem alterar o documento.

## Entregas e verificação

Ordem recomendada: **precisão → cubo → recorte/cache do hover → tampas/divisórias →
oclusão integrada do hover e clique**. Fazer o ensaio da seção antes de assumir o
escopo final das duas últimas entregas. Cada entrega recebe critérios de aceite
próprios; uma correção parcial de recorte não encerra a caixa de hover.

Para mudanças de código, seguir fmt → clippy → testes, com `--locked` nos comandos
que aceitam essa opção. Executar os checks do workspace e testes de comportamento
para configuração, compatibilidade, orientação, recorte e rejeição de resultados
antigos. Acrescentar verificações GPU/capturas: testes numéricos não demonstram
ausência de z-fighting ou oclusão correta. Reutilizar `--render-ui-scenario`, ampliando
os cenários `settings-drawing`, `section` e hover; usar destinos novos.

Caso a implementação use o kernel, validar também com OCCT e target separado,
conforme `AGENTS.md`. O planejamento não exige novas dependências; a triangulação
de tampas deve ser avaliada antes de decidir por uma. Atualizar `THIRD_PARTY.md`
se uma dependência direta for introduzida.

Não há necessidade prevista de alterar o formato 49: a precisão já é persistida,
e cubo/seção/hover são controles visuais. Atualizar `CHANGELOG.md`, `docs/INTERFACE.md`
e `docs/VALIDATION.md` em cada entrega. Definir a versão do aplicativo conforme o
escopo entregue e só marcar caixas de `AJUSTES.md` com evidências de aceite.

## Execução — 04/11/2026

- **0.37.1:** precisão linear única, padrão 0,1 mm e unidade sem remapear o valor físico.
- **0.37.2:** clique nas faces do cubo conserva o alto predominante, em quartos de volta.
- **0.38.0:** tampas no plano e divisórias por corpo/ocorrência, com geração assíncrona e preservação de cavidades.
- **0.38.1:** hover recortado com profundidade e seleção coerente com as tampas. Evidências e resultados finais em `VALIDATION.md`.

A implementação usa a malha de apresentação para as tampas; não foi necessária dependência nova nem alteração do formato 49. Tampas não se tornam referências CAD selecionáveis.
