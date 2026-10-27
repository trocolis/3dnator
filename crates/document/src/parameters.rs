//! Expressões dimensionais pequenas, sem executar código ou acessar recursos externos.
use crate::DocumentError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
type Result<T> = std::result::Result<T, DocumentError>;

pub fn resolve_sketch(sketch: &mut nator_sketch::Sketch, parameters: &Parameters) -> Result<()> {
    use nator_sketch::ConstraintKind;
    for (id, expression) in &sketch.dimension_expressions {
        let constraint = sketch
            .constraints
            .iter_mut()
            .find(|c| c.id == *id)
            .ok_or_else(|| DocumentError("Expressão sem cota correspondente.".into()))?;
        let dimension = if matches!(constraint.kind, ConstraintKind::Angle { .. }) {
            Dimension::Angle
        } else {
            Dimension::Length
        };
        let value = parameters.evaluate(expression, dimension)?;
        match &mut constraint.kind {
            ConstraintKind::Distance { value: target, .. }
            | ConstraintKind::DeltaX { value: target, .. }
            | ConstraintKind::DeltaY { value: target, .. }
            | ConstraintKind::Angle { value: target, .. }
            | ConstraintKind::Radius { value: target, .. } => *target = value,
            _ => {
                return Err(DocumentError(
                    "A expressão exige uma cota dimensional.".into(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dimension {
    Scalar,
    Length,
    Angle,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub expression: String,
    pub dimension: Dimension,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Parameters {
    pub entries: BTreeMap<String, Parameter>,
}
#[derive(Debug, Clone, Copy)]
pub struct Quantity {
    pub value: f64,
    length: i8,
    angle: i8,
}
impl Quantity {
    fn scalar(value: f64) -> Self {
        Self {
            value,
            length: 0,
            angle: 0,
        }
    }
    fn dimension(self) -> (i8, i8) {
        (self.length, self.angle)
    }
    fn finite(self) -> Result<Self> {
        if self.value.is_finite() {
            Ok(self)
        } else {
            Err(DocumentError(
                "A expressão produziu um valor não finito.".into(),
            ))
        }
    }
    fn expected(mut self, dimension: Dimension) -> Result<Self> {
        let expected = match dimension {
            Dimension::Scalar => (0, 0),
            Dimension::Length => (1, 0),
            Dimension::Angle => (0, 1),
        };
        if self.dimension() == (0, 0) && expected != (0, 0) {
            self.length = expected.0;
            self.angle = expected.1;
            if dimension == Dimension::Angle {
                self.value = self.value.to_radians();
            }
        }
        if self.dimension() != expected {
            return Err(DocumentError(
                "A expressão tem unidades incompatíveis com o campo.".into(),
            ));
        }
        self.finite()
    }
}
impl Parameters {
    pub fn validate(&self) -> Result<()> {
        if self.entries.len() > 256 {
            return Err(DocumentError("Limite de 256 parâmetros por peça.".into()));
        }
        for (name, parameter) in &self.entries {
            validate_name(name)?;
            self.evaluate(&parameter.expression, parameter.dimension)?;
        }
        // Avaliar pelo nome também detecta o ciclo de um parâmetro que se refere a si mesmo.
        for name in self.entries.keys() {
            let mut context = Context {
                table: self,
                memo: BTreeMap::new(),
                visiting: HashSet::new(),
            };
            context.variable(name)?;
        }
        Ok(())
    }
    pub fn set(&mut self, name: String, expression: String, dimension: Dimension) -> Result<()> {
        validate_name(&name)?;
        let mut candidate = self.clone();
        candidate.entries.insert(
            name,
            Parameter {
                expression,
                dimension,
            },
        );
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn evaluate(&self, expression: &str, dimension: Dimension) -> Result<f64> {
        let mut context = Context {
            table: self,
            memo: BTreeMap::new(),
            visiting: HashSet::new(),
        };
        Ok(context.expression(expression)?.expected(dimension)?.value)
    }
}
fn validate_name(name: &str) -> Result<()> {
    if name.len() > 128
        || name.is_empty()
        || !name
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_alphabetic() || i > 0 && c.is_ascii_digit())
        || unit(name).is_some()
        || matches!(
            name,
            "pi" | "π" | "sin" | "cos" | "tan" | "sqrt" | "abs" | "min" | "max"
        )
    {
        return Err(DocumentError(
            "Nome de parâmetro inválido ou reservado.".into(),
        ));
    }
    Ok(())
}
struct Context<'a> {
    table: &'a Parameters,
    memo: BTreeMap<String, Quantity>,
    visiting: HashSet<String>,
}
impl Context<'_> {
    fn variable(&mut self, name: &str) -> Result<Quantity> {
        if name == "pi" || name == "π" {
            return Ok(Quantity::scalar(std::f64::consts::PI));
        }
        if let Some(value) = self.memo.get(name) {
            return Ok(*value);
        }
        if self.visiting.len() > 128 || !self.visiting.insert(name.into()) {
            return Err(DocumentError(format!(
                "Dependência circular ou profunda no parâmetro '{name}'."
            )));
        }
        let parameter = self
            .table
            .entries
            .get(name)
            .ok_or_else(|| DocumentError(format!("Parâmetro '{name}' inexistente.")))?
            .clone();
        let value = self
            .expression(&parameter.expression)?
            .expected(parameter.dimension)?;
        self.visiting.remove(name);
        self.memo.insert(name.into(), value);
        Ok(value)
    }
    fn expression(&mut self, text: &str) -> Result<Quantity> {
        if text.is_empty() || text.len() > 4096 {
            return Err(DocumentError("Expressão vazia ou longa demais.".into()));
        }
        let mut parser = Parser {
            characters: text.chars().collect(),
            offset: 0,
            depth: 0,
            context: self,
        };
        let result = parser.sum()?;
        parser.space();
        if parser.offset != parser.characters.len() {
            return Err(DocumentError(
                "Texto inesperado no final da expressão.".into(),
            ));
        }
        result.finite()
    }
}
struct Parser<'a, 'b> {
    characters: Vec<char>,
    offset: usize,
    depth: usize,
    context: &'a mut Context<'b>,
}
fn unit(name: &str) -> Option<Quantity> {
    let (value, length, angle) = match name {
        "mm" => (1., 1, 0),
        "cm" => (10., 1, 0),
        "m" => (1000., 1, 0),
        "in" => (25.4, 1, 0),
        "rad" => (1., 0, 1),
        "deg" | "°" => (std::f64::consts::PI / 180., 0, 1),
        _ => return None,
    };
    Some(Quantity {
        value,
        length,
        angle,
    })
}
impl Parser<'_, '_> {
    fn space(&mut self) {
        while self
            .characters
            .get(self.offset)
            .is_some_and(|c| c.is_whitespace())
        {
            self.offset += 1;
        }
    }
    fn eat(&mut self, character: char) -> bool {
        self.space();
        if self.characters.get(self.offset) == Some(&character) {
            self.offset += 1;
            true
        } else {
            false
        }
    }
    fn identifier(&mut self) -> String {
        let start = self.offset;
        while self
            .characters
            .get(self.offset)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_' || *c == '°')
        {
            self.offset += 1;
        }
        self.characters[start..self.offset].iter().collect()
    }
    fn sum(&mut self) -> Result<Quantity> {
        let mut left = self.product()?;
        loop {
            let sign = if self.eat('+') {
                1.
            } else if self.eat('-') {
                -1.
            } else {
                break;
            };
            let right = self.product()?;
            if left.dimension() != right.dimension() {
                return Err(DocumentError(
                    "Soma e subtração exigem unidades compatíveis; use unidades explícitas.".into(),
                ));
            }
            left.value += sign * right.value;
            left = left.finite()?;
        }
        Ok(left)
    }
    fn product(&mut self) -> Result<Quantity> {
        let mut left = self.unary()?;
        loop {
            let division = if self.eat('*') {
                false
            } else if self.eat('/') {
                true
            } else {
                break;
            };
            let right = self.unary()?;
            if division && right.value == 0. {
                return Err(DocumentError("Divisão por zero na expressão.".into()));
            }
            let sign = if division { -1 } else { 1 };
            let length = i16::from(left.length) + sign * i16::from(right.length);
            let angle = i16::from(left.angle) + sign * i16::from(right.angle);
            if length.abs() > 32 || angle.abs() > 32 {
                return Err(DocumentError(
                    "Dimensão física da expressão excede o limite.".into(),
                ));
            }
            left = Quantity {
                value: if division {
                    left.value / right.value
                } else {
                    left.value * right.value
                },
                length: length as i8,
                angle: angle as i8,
            }
            .finite()?;
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Quantity> {
        if self.depth >= 64 {
            return Err(DocumentError("Expressão aninhada demais.".into()));
        }
        self.depth += 1;
        let result = self.unary_value();
        self.depth -= 1;
        result
    }
    fn unary_value(&mut self) -> Result<Quantity> {
        if self.eat('+') {
            return self.unary();
        }
        if self.eat('-') {
            let mut value = self.unary()?;
            value.value = -value.value;
            return Ok(value);
        }
        let mut value = self.atom()?;
        if self.eat('^') {
            let power = self.unary()?;
            if power.dimension() != (0, 0)
                || power.value.abs() > 8.
                || (power.value - power.value.round()).abs() > 1e-9
            {
                return Err(DocumentError(
                    "O expoente deve ser um inteiro entre -8 e 8.".into(),
                ));
            }
            let exponent = power.value as i16;
            let length = i16::from(value.length) * exponent;
            let angle = i16::from(value.angle) * exponent;
            if length.abs() > 32 || angle.abs() > 32 {
                return Err(DocumentError("Dimensão física excessiva.".into()));
            }
            value = Quantity {
                value: value.value.powi(i32::from(exponent)),
                length: length as i8,
                angle: angle as i8,
            }
            .finite()?;
        }
        Ok(value)
    }
    fn atom(&mut self) -> Result<Quantity> {
        self.space();
        if self.depth > 64 {
            return Err(DocumentError("Expressão aninhada demais.".into()));
        }
        if self.eat('(') {
            self.depth += 1;
            let value = self.sum()?;
            if !self.eat(')') {
                return Err(DocumentError("Parêntese de fechamento ausente.".into()));
            }
            self.depth -= 1;
            return Ok(value);
        }
        let c = self
            .characters
            .get(self.offset)
            .copied()
            .ok_or_else(|| DocumentError("Expressão incompleta.".into()))?;
        if c.is_ascii_digit() || c == '.' || c == ',' {
            let start = self.offset;
            while let Some(c) = self.characters.get(self.offset).copied() {
                if c.is_ascii_digit()
                    || matches!(c, '.' | ',' | 'e' | 'E')
                    || matches!(c, '+' | '-')
                        && self.offset > start
                        && matches!(self.characters[self.offset - 1], 'e' | 'E')
                {
                    self.offset += 1;
                } else {
                    break;
                }
            }
            let number = self.characters[start..self.offset]
                .iter()
                .collect::<String>()
                .replace(',', ".");
            let mut value = Quantity::scalar(
                number
                    .parse::<f64>()
                    .map_err(|_| DocumentError("Número inválido na expressão.".into()))?,
            )
            .finite()?;
            self.space();
            let saved = self.offset;
            let name = self.identifier();
            if let Some(unit) = unit(&name) {
                value = Quantity {
                    value: value.value * unit.value,
                    ..unit
                };
            } else {
                self.offset = saved;
            }
            return value.finite();
        }
        if c.is_alphabetic() || c == '_' {
            let name = self.identifier();
            if !self.eat('(') {
                return self.context.variable(&name);
            }
            self.depth += 1;
            let first = self.sum()?;
            let second = if self.eat(';') {
                Some(self.sum()?)
            } else {
                None
            };
            if !self.eat(')') {
                return Err(DocumentError(
                    "Função incompleta; use ponto e vírgula entre argumentos.".into(),
                ));
            }
            self.depth -= 1;
            return function(&name, first, second);
        }
        Err(DocumentError("Caractere inválido na expressão.".into()))
    }
}
fn function(name: &str, first: Quantity, second: Option<Quantity>) -> Result<Quantity> {
    let result = match (name, second) {
        ("sin" | "cos" | "tan", None) if matches!(first.dimension(), (0, 0) | (0, 1)) => {
            Quantity::scalar(match name {
                "sin" => first.value.sin(),
                "cos" => first.value.cos(),
                _ => first.value.tan(),
            })
        }
        ("abs", None) => Quantity {
            value: first.value.abs(),
            ..first
        },
        ("sqrt", None) if first.value >= 0. && first.length % 2 == 0 && first.angle % 2 == 0 => {
            Quantity {
                value: first.value.sqrt(),
                length: first.length / 2,
                angle: first.angle / 2,
            }
        }
        ("min" | "max", Some(second)) if first.dimension() == second.dimension() => Quantity {
            value: if name == "min" {
                first.value.min(second.value)
            } else {
                first.value.max(second.value)
            },
            ..first
        },
        _ => {
            return Err(DocumentError(format!(
                "Função '{name}' desconhecida ou com argumentos/unidades incompatíveis."
            )));
        }
    };
    result.finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn angular_sketch_expressions_resolve_as_angles_and_recalculate()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use nator_sketch::{ConstraintKind, Curve, Plane, Sketch, solver};
        let mut s = Sketch::new("Linha cotada", Plane::xy());
        let line = s.add_line([0., 0.], [10., 0.], false)?;
        let Curve::Line { start, end } = s.entity(line)?.curve else {
            return Err("Linha ausente".into());
        };
        s.add_constraint(
            ConstraintKind::Fixed {
                point: start,
                position: [0., 0.],
            },
            false,
        )?;
        let length = s.add_constraint(
            ConstraintKind::Distance {
                a: start,
                b: end,
                value: 10.,
            },
            false,
        )?;
        let angle = s.add_constraint(
            ConstraintKind::Angle {
                a: start,
                b: end,
                value: 0.,
            },
            false,
        )?;
        s.dimension_expressions.insert(length, "20 mm".into());
        s.dimension_expressions.insert(angle, "inclinacao".into());
        let mut p = Parameters::default();
        for value in ["30 deg", "90", "-0.5 rad"] {
            p.set("inclinacao".into(), value.into(), Dimension::Angle)?;
            resolve_sketch(&mut s, &p)?;
            let report = solver::solve(&s)?;
            assert_ne!(report.status, solver::Status::Conflict);
            let (a, b) = report.sketch.line(line)?;
            let expected = p.evaluate("inclinacao", Dimension::Angle)?;
            assert!(((b[1] - a[1]).atan2(b[0] - a[0]) - expected).abs() < 1e-6);
            assert!((nator_sketch::distance(a, b) - 20.).abs() < 1e-6);
        }
        s.dimension_expressions.insert(angle, "5 mm".into());
        assert!(resolve_sketch(&mut s, &p).is_err());
        Ok(())
    }
    #[test]
    fn expressions_preserve_units_and_support_portuguese_decimals() -> Result<()> {
        let mut p = Parameters::default();
        p.set("largura".into(), "60 mm".into(), Dimension::Length)?;
        assert_eq!(p.evaluate("largura / 2", Dimension::Length)?, 30.);
        assert_eq!(p.evaluate("largura - 2,5 cm", Dimension::Length)?, 35.);
        assert!((p.evaluate("sin(90 deg)", Dimension::Scalar)? - 1.).abs() < 1e-12);
        assert_eq!(
            p.evaluate("sqrt((3 mm)^2 + (4 mm)^2)", Dimension::Length)?,
            5.
        );
        assert!(p.evaluate("largura * largura", Dimension::Length).is_err());
        assert!(p.evaluate("largura + 3", Dimension::Length).is_err());
        Ok(())
    }
    #[test]
    fn circular_changes_are_rejected_atomically() -> Result<()> {
        let mut p = Parameters::default();
        p.set("a".into(), "60 mm".into(), Dimension::Length)?;
        p.set("b".into(), "a / 2".into(), Dimension::Length)?;
        assert!(
            p.set("a".into(), "b * 2".into(), Dimension::Length)
                .is_err()
        );
        assert_eq!(p.evaluate("a", Dimension::Length)?, 60.);
        Ok(())
    }
}
