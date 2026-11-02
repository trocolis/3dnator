//! Ajuda: um tutorial curto por tarefa, com busca no título e no texto. Por ora só em
//! português, como a documentação; os atalhos vêm da configuração em vigor.

pub(super) struct Topic {
    pub title: &'static str,
    pub steps: &'static [&'static str],
}

pub(super) const TOPICS: &[Topic] = &[
    Topic {
        title: "Criar uma peça e um rascunho",
        steps: &[
            "Na aba Montagem, Nova peça cria uma peça vazia e entra na construção dela.",
            "Na aba Construção, Novo rascunho pede o plano: clique num plano da origem (XY, XZ, YZ), num plano de trabalho ou numa face plana de um corpo.",
            "Sobre uma face, o rascunho já traz as linhas dela em amarelo: são referências fixas, que delimitam áreas e servem de apoio para restrições e cotas.",
            "Desenhe com Linha (L), Retângulo (R), Círculo (C) e Arco (A). Digite a medida durante o desenho para usar um valor exato; Enter confirma.",
            "Dimensionar (D) cota linhas, distâncias, raios e ângulos. O balão no canto mostra quantos graus de liberdade ainda restam.",
            "Pontas novas a menos da precisão do mouse de um vértice existente se ligam a ele sozinhas, evitando frestas na impressão.",
            "Concluir rascunho, na faixa, volta à construção da peça.",
        ],
    },
    Topic {
        title: "Extrudar",
        steps: &[
            "Na aba Construção, Extrusão (E) mostra as áreas sob o cursor: regiões do rascunho, partes de uma face plana e faces sem desenho.",
            "Clique na área. A seta aparece com o campo da distância já em foco: digite o valor e confirme com Enter, ou arraste a seta.",
            "Com a operação automática, começar numa face e sair dela une ao corpo; entrar no material corta.",
            "Na união, as faces novas que continuam faces existentes no mesmo plano viram uma só área (Juntar áreas coplanares, no painel).",
            "Distância zero só divide a face, sem mudar o volume: útil para pintar ou extrudar uma parte dela depois.",
        ],
    },
    Topic {
        title: "Arredondar e chanfrar",
        steps: &[
            "Arredondar (F) e Chanfrar (C), na aba Construção, pedem as arestas: clique em cada uma, ou perto de um vértice para escolher todas as que chegam nele.",
            "Informe o raio ou a distância no painel. A prévia aparece antes de confirmar.",
            "No rascunho, Arredondar (O) e Chanfrar (H) tratam cantos de linhas; clique nos vértices para escolher só alguns cantos.",
        ],
    },
    Topic {
        title: "Montar peças",
        steps: &[
            "Na aba Montagem, Mover (M) posiciona peças por setas, planos e anéis de rotação, ou ponto a ponto entre faces, arestas e vértices.",
            "Importar traz STEP, STL ou 3MF. Como referência, o modelo fica na pasta Referências: pode ser movido, ocultado e medido, mas não editado. No menu dele, Criar peça o transforma numa peça.",
            "Arraste uma peça para outra na árvore para torná-la subpeça; arraste um corpo para outra peça para transferi-lo.",
            "Medir, na barra de navegação, mostra na cena a medida de uma aresta, a distância entre dois elementos ou o ângulo entre eles. Duplo clique no segundo alvo mostra as distâncias em X, Y e Z.",
        ],
    },
    Topic {
        title: "Exportar para imprimir",
        steps: &[
            "Na aba Impressão, Verificar impressão aponta paredes finas, balanços e lascas antes de exportar.",
            "Exportar para impressão grava STL ou 3MF com a qualidade escolhida. A exportação nunca sobrescreve um arquivo existente.",
            "STEP, na aba Montagem, grava a geometria exata com as cores, para outros programas de CAD.",
        ],
    },
    Topic {
        title: "Configurações",
        steps: &[
            "Aparência escolhe o tema (claro, escuro ou o do sistema) e o idioma.",
            "Desenho guarda, no projeto, a unidade (mm ou polegada), a precisão do mouse e as casas decimais exibidas.",
            "Atalhos troca a tecla de cada ação: clique nela e pressione a nova combinação.",
        ],
    },
];

/// Texto sem acento e em minúsculas, para a busca achar "extrusao" e "Extrusão".
pub(super) fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'î' | 'ì' | 'ï' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' => 'o',
            'ú' | 'û' | 'ù' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// O tópico tem todas as palavras da busca, no título ou nos passos.
pub(super) fn matches(topic: &Topic, query: &str) -> bool {
    let haystack = fold(&format!("{} {}", topic.title, topic.steps.join(" ")));
    fold(query)
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_ignores_accents_case_and_word_order() {
        let found = TOPICS
            .iter()
            .filter(|t| matches(t, "EXTRUSAO seta"))
            .map(|t| t.title)
            .collect::<Vec<_>>();
        assert_eq!(found, ["Extrudar"]);
        assert_eq!(
            TOPICS.iter().filter(|t| matches(t, "")).count(),
            TOPICS.len()
        );
        assert!(!TOPICS.iter().any(|t| matches(t, "xyzzy")));
    }
}
