# Avisos de dependências de terceiros e licenças

O código-fonte próprio do **3dnator** é distribuído sob os termos da licença **MIT** (conforme [LICENSE](LICENSE)).
As bibliotecas e recursos de terceiros incorporados ou vinculados mantêm suas licenças e direitos autorais próprios, conforme detalhado a seguir.

---

## 1. Open CASCADE Technology (OCCT)

### Aviso proeminente (Prominent Notice)

> **Notice:** This software makes use of and is based on facilities provided by the Open CASCADE Technology software (<https://dev.opencascade.org> e <https://github.com/Open-Cascade-SAS/OCCT>).

### Licença e termos de distribuição

O Open CASCADE Technology (OCCT) é distribuído sob a **GNU Lesser General Public License (LGPL) versão 2.1**, acompanhada da **Open CASCADE Technology Exception (versão 1.0)**.

A ponte própria de integração do 3dnator (`nator_occt`) consome o OCCT através de **vinculação dinâmica** (`.so` em sistemas Linux e `.dll` em sistemas Windows). As bibliotecas permanecem separadas e substituíveis por versões compatíveis. Os textos integrais distribuídos pelo SDK estão em [licenses/OCCT/LICENSE_LGPL_21.txt](licenses/OCCT/LICENSE_LGPL_21.txt) e [licenses/OCCT/OCCT_LGPL_EXCEPTION.txt](licenses/OCCT/OCCT_LGPL_EXCEPTION.txt), incluídos pelos scripts de empacotamento.

O código-fonte original e completo do Open CASCADE Technology pode ser obtido gratuitamente em seu portal oficial:
- Portal oficial: <https://dev.opencascade.org/>
- Repositório de código-fonte: <https://github.com/Open-Cascade-SAS/OCCT>

### Texto da Open CASCADE Technology Exception (versão 1.0)

```text
Open CASCADE exception (version 1.0) to GNU LGPL version 2.1.

The object code (i.e. not a source) form of a "work that uses the Library"
can incorporate material from a header file that is part of the Library.
As a special exception to the GNU Lesser General Public License version 2.1,
you may distribute such object code incorporating material from header files
provided with the Open CASCADE Technology libraries (including code of CDL
generic classes) under terms of your choice, provided that you give
prominent notice in supporting documentation to this code that it makes use
of or is based on facilities provided by the Open CASCADE Technology software.
```

---

## 2. Dependências diretas do projeto

| Dependência | Versão | Licença | Uso no projeto | Repositório |
| --- | --- | --- | --- | --- |
| `egui` / `eframe` | 0.36.2 | MIT OR Apache-2.0 | Interface gráfica e gerenciamento de janela | <https://github.com/emilk/egui> |
| `wgpu` | 30.0 | MIT OR Apache-2.0 | Renderização acelerada por GPU (Vulkan, DX12, Metal) | <https://github.com/gfx-rs/wgpu> |
| `serde` / `serde_json` | 1.0 | MIT OR Apache-2.0 | Serialização e manifesto de projetos `.3dnator` | <https://github.com/serde-rs/serde> |
| `uuid` | 1.27 | MIT OR Apache-2.0 | Geração de identificadores únicos para projetos | <https://github.com/uuid-rs/uuid> |
| `zip` | 2.x | MIT | Leitura e gravação de pacotes comprimidos `.3dnator` | <https://github.com/zip-rs/zip2> |
| `tempfile` | 3.x | MIT OR Apache-2.0 | Gravação atômica segura de arquivos de projeto | <https://github.com/Stebalien/tempfile> |
| `rfd` | 0.17 | MIT | Diálogos nativos de seleção e salvamento de arquivos | <https://github.com/PolyMeilex/rfd> |
| `png` | 0.18 | MIT OR Apache-2.0 | Decodificação e codificação de capturas de tela e folhas | <https://github.com/image-rs/image-png> |
| `pollster` | 0.4 | MIT OR Apache-2.0 | Execução de tarefas assíncronas em contextos síncronos | <https://github.com/zesterer/pollster> |
| `Open CASCADE Technology` | 7.x | LGPL-2.1 with Exception | Núcleo de modelagem B-rep, HLR, XCAF e intercâmbio STEP | <https://dev.opencascade.org> |

---

## 3. Fontes tipográficas incorporadas

A biblioteca gráfica `eframe`/`epaint` embute cópias de fontes tipográficas para renderização padrão de textos na interface:
- **Ubuntu Font:** Distribuída sob a licença *Ubuntu Font Licence 1.0*.
- **Noto Fonts:** Distribuída sob a licença *SIL Open Font License (OFL) 1.1*.

---

## 4. Dependências transitivas do ecossistema Rust

Esta lista resume famílias de licenças das dependências Rust; não é um inventário completo por pacote e versão. Antes de uma release pública, conferir os avisos e textos das crates e fontes incorporadas, além das dependências nativas efetivamente incluídas conforme `NATIVE_DEPENDENCIES.txt`:

- **Licenças permissivas padrão (MIT / Apache-2.0):** Utilizadas pela grande maioria das crates auxiliares do ecossistema Rust (estruturas de dados, gerenciamento assíncrono, vinculações de janelas `winit`, geração de shaders `naga`, abstrações Vulkan `ash`, etc.).
- **Zlib / libpng:** Utilizadas pelas rotinas de compressão e manipulação gráfica (`miniz_oxide`, `crc32fast`).
- **BSD-2-Clause / BSD-3-Clause / ISC:** Utilizadas por módulos específicos de tratamento de ponto flutuante e chamadas de sistema.
- **Boost Software License 1.0 (BSL-1.0):** Utilizada por rotinas numéricas e de hashing.
- **Unlicense / 0BSD:** Dependências de domínio público ou sem restrições.
