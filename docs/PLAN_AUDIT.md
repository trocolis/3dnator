# Auditoria do plano — 26/10/2026

O plano inicial tem suas funcionalidades principais implementadas e verificadas localmente. **O aceite completo continua aberto**: Windows nativo, Ubuntu com OCCT 7.6, hardware físico de 8 GB e interação real ainda não foram validados. A afirmação anterior de conclusão de todas as frentes não demonstrava esses critérios.

**Repriorização posterior do usuário:** a validação Windows está adiada enquanto refinamos funcionalidades e interface no Linux. A CI Windows passa a ser opcional, acionada manualmente com `validate_windows`; sua ausência não bloqueia os incrementos atuais. Os critérios acima são mantidos para retomada futura, sem declarar testes não executados.

## Evidências por etapa

| Etapa | Evidência executada nesta auditoria | Aceite restante |
| --- | --- | --- |
| 1. Base técnica | Formatação; Clippy sem avisos; 87 testes sem OCCT e 119 com OCCT; compilação desktop debug/release no Linux; capturas wgpu; empacotamento e relocação | Executar CI Windows/Ubuntu; interação em janela; hardware de referência conforme limites em `VALIDATION.md` |
| 2. Primeira peça | `--validate-project`, `--validate-sketch`, `--validate-part`; recálculo, persistência, recuperação, desfazer/refazer e ida/volta STEP/STL | Repetir o fluxo com mouse, teclado e diálogos nas duas plataformas |
| 3. Construção mecânica | Testes `mechanical_occt`, `parametric_occt` e `naming_occt` no workspace com OCCT | Conferir prévias, confirmação e cancelamento em janela |
| 4. Montagem | Testes de hierarquia, clones, grupos e espelhamento; `--validate-editor` e `--validate-step` | Repetir posicionamento, padrões e espelhamento na janela |
| 5. Documentação técnica | Testes de desenhos/cotas; ensaios de folha PNG/PDF, HLR e cortes do B-rep; imagens do editor e da folha inspecionadas | Conferir criação/edição da folha e arraste de cotas na janela |

Os testes com OCCT incluem a base comum; as duas contagens não representam conjuntos disjuntos. Evidências locais estão em `target/codex-validation/audit-20261026/`, ignoradas pelo Git. Os seis ensaios CLI foram executados com o pacote movido para `relocated/`, usando `env -i PATH=/usr/bin:/bin`. Todos geraram relatório `status: passed`.

Também foi criado o pacote release `package-release.tar.gz`, extraído em `extracted/` e executado nesse novo local com ambiente limpo. `--validate`, `--validate-part` e `--validate-step` passaram novamente a partir do arquivo extraído; relatórios em `release-validate/`, `release-part/` e `release-step/`.

O critério analítico da primeira peça produziu 14.375,365076 mm³, contra 14.375,365076 mm³ esperados, e STL fechado de 656 triângulos. STEP de montagem preservou volume de 6.000 mm³ por corpo, nomes, posições, definições compartilhadas e cores.

## Pendências corrigidas nesta auditoria

- Os scripts removiam recursivamente o destino informado. Agora exigem destino e arquivo comprimido novos, preservando dados existentes e recusando opções incompletas.
- No Linux, `--occt-lib-dir` não ajudava `ldd` a resolver o SDK; bibliotecas ausentes eram ignoradas. Agora o diretório participa da resolução, ausências abortam antes da criação da saída, e os SONAMEs são copiados sem links para a instalação original. TBB transitivo também é incluído quando presente.
- No Windows, a cópia apenas de `TK*.dll` omitiria dependências como TBB e FreeType. Agora a coleta percorre os imports com `dumpbin`, incluindo os transitivos e rejeitando ausências. **Este caminho ainda depende de execução Windows; não foi declarado aprovado localmente.**
- Faltava o texto integral LGPL citado na documentação. Os textos fornecidos pelo SDK OCCT agora estão versionados em `licenses/OCCT/` e entram nos dois pacotes.
- O README do pacote agora explica sua execução e seus requisitos, em vez de instruir o usuário a compilar o repositório. O manifesto registra dependências incluídas e requisitos do host.
- A CI agora compila o desktop com OCCT, testa o workspace inteiro, empacota, move o pacote e executa os ensaios sem o SDK no PATH. Antes verificava OCCT apenas sem desktop e não testava o pacote. As regressões de preservação de arquivos também entram na CI.
- Corrigidas a versão interna obsoleta do formato no guia (11), a lista de pendências desatualizada e as afirmações de portabilidade e inventário completo de licenças sem evidência suficiente.
- Definidos limites numéricos provisórios de desempenho e repetido o benchmark release. O adaptador disponível foi llvmpipe; os resultados não substituem a medição em GPU integrada física.

## Como fechar o aceite

1. Em runners nativos Windows e Ubuntu 24.04, executar `.github/workflows/ci.yml`, guardar os logs e conferir os relatórios dos pacotes. O repositório não possui remoto e não foi publicado; preparar o workflow não equivale a executá-lo.
2. Na máquina física de 8 GB com GPU integrada, rodar o benchmark release três vezes, conferir o adaptador registrado e confrontar as medianas com os limites de `VALIDATION.md`. Registrar CPU/GPU, sistema, RAM e driver. Não usar resultados de llvmpipe para aprovar navegação em GPU física.
3. Nas duas plataformas, executar com teclado e mouse: criar o suporte cotado; extrudar/furar/arredondar; alterar uma cota; desfazer/refazer; salvar/reabrir por diálogo; exportar STEP/STL; montar instâncias e espelhar um conjunto; gerar folha com dois cortes e cota associativa; arrastar a cota; mudar o modelo; exportar PNG/PDF. Conferir também cancelamento por Esc e fechamento de projeto modificado.
4. Registrar resultados reais e só então marcar o aceite completo no ROADMAP. Hélice real, raio variável, restrições contínuas de montagem, cores por face e arquivos STEP externos estão fora do escopo inicial; não são pendências desta entrega.

Uma release pública é uma decisão separada. Antes dela, revisar os avisos e textos das dependências/fontes incorporadas e das bibliotecas nativas efetivamente distribuídas. `THIRD_PARTY.md` é um resumo, não essa revisão completa. Não houve tag, publicação, envio remoto ou execução de CI externa nesta auditoria.
