# Arquitetura do aplicativo

Estrutura aprovada para um workspace Rust. O incremento 0.2.0 implementa `geometry`, `document`, `commands`, persistência/intercâmbio em `project_io` e visualização em `renderer`, além da ponte opcional em `kernel`. A interface experimental permanece em `apps/prototype/src/desktop`, com composição visual em módulo próprio. Rascunhos, histórico paramétrico e a extração da interface para seu crate final ainda não foram implementados.

## Módulos

| Módulo | Responsabilidade |
| --- | --- |
| `geometry` | Coordenadas, unidades, transformações, identificadores e tipos geométricos compartilhados |
| `sketch` | Entidades 2D, restrições, solução e identificação de regiões |
| `kernel` | Integração com OCCT, sólidos, operações e tesselação, através de API própria |
| `document` | Peças, corpos, instâncias, grupos, parâmetros e histórico paramétrico |
| `commands` | Ações de edição, transações, desfazer/refazer e coordenação do recálculo |
| `project_io` | Formato `.3dnator`, migrações, recuperação e importação/exportação |
| `renderer` | Visualização em wgpu, seleção gráfica e destaques |
| `ui` | Painéis, ferramentas interativas, atalhos e traduções |
| `app` | Inicialização e integração com Linux e Windows |

## Limites e dependências

- A interface solicita alterações por comandos, em vez de modificar diretamente o documento.
- As regras geométricas e de negócio não dependem da interface ou do renderizador. Geometria, histórico e persistência devem poder ser verificados sem abrir uma janela.
- `geometry` permanece independente dos módulos superiores. `sketch` e `kernel` não dependem de `document`, `commands`, `ui` ou `app`.
- `document` usa os tipos e APIs próprios de geometria, rascunhos e núcleo; não recebe tipos públicos de OCCT.
- `commands` coordena as alterações do documento e o recálculo. `project_io` usa o documento e as APIs internas para codificar, decodificar e converter dados, sem depender da interface.
- `renderer` recebe dados de apresentação derivados do documento, sem se tornar a fonte da geometria precisa.
- `ui` apresenta o estado, conduz interações e solicita comandos e operações de arquivo; `app` conecta esses serviços ao ciclo de vida da aplicação.
- A definição dos crates concretos e de seus contratos deve respeitar esses limites, sem dependências circulares.

## Operações, prévias e erros

- No incremento atual, o documento usa identificadores tipados e geometria de malha imutável compartilhada por Arc. Transformações de corpos são locais à peça; instâncias aplicam uma transformação adicional na montagem.
- `Session` publica uma edição somente após validar sua cópia candidata; desfazer/refazer mantém até 100 snapshots leves. A revisão da sessão cresce também ao desfazer/refazer, enquanto o identificador do estado permite reconhecer o último salvamento.
- Cenas e buffers CPU são preparados na thread de trabalho. O resultado de importação identifica a aba e a revisão originais, e uma cena antiga não substitui a revisão atual. A interface mantém apenas os buffers GPU da aba ativa.
- O formato de projeto ainda não serializa sólidos CAD nem histórico paramétrico. O salvamento atual preserva peças, corpos de malha, referências de instâncias, nomes, visibilidade e transformações.

- Uma operação confirmada é atômica: sua conclusão atualiza o documento e adiciona uma ação à sequência de desfazer, ou a falha preserva o documento anterior.
- Prévias e edições pendentes usam estado separado até a confirmação. Uma edição paramétrica com falha permanece pendente para correção, supressão da etapa afetada ou cancelamento, conforme as regras do editor.
- Um arraste completo e uma operação sobre vários elementos podem compor uma única transação de edição.
- Cálculos demorados ficam fora da thread da interface; resultados devem ser associados à revisão que os originou, impedindo que um resultado antigo sobrescreva uma edição mais recente.
- Erros da integração nativa devem ser convertidos em erros da API própria do núcleo. Gestão de memória e detalhes de interoperabilidade ficam isolados nessa integração.

## Próxima validação

OCCT é o candidato prioritário, ainda sem integração validada. A etapa de base deve verificar operações representativas, STEP, tesselação e distribuição em Linux e Windows antes de consolidar a dependência.
