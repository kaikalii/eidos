use std::{env, fs};

use derive_more::{Display, From};
use enum_iterator::{all, cardinality, Sequence};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{color::Color, field::*, function::*, utils::resources_path};

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
    From,
    Sequence,
    Serialize,
    Deserialize,
)]
pub enum Word {
    // Numbers
    /// 0
    To,
    /// 1
    Ti,
    /// 2
    Tu,
    /// 5
    Ta,
    /// 10
    Te,

    // Scalars
    /// X
    Se,
    /// Y
    Si,
    /// Scalar variable
    Su,

    // Vectors
    /// Unit vector X
    Ke,
    /// Unit vector Y
    Ki,
    /// Vector variable
    Ku,

    // Inputs
    /// Elevation
    Wi,
    /// Density
    Ro,
    /// Temperature
    Lu,
    /// Disorder
    Ko,
    /// Read
    Re,

    // Outputs
    /// Gravity
    Vu,
    /// Force
    Wu,
    /// Heat
    Lo,
    /// Order
    Mu,
    /// Anchor
    Nu,
    /// Write
    Ri,

    // Operators
    /// Add
    Ma,
    /// Multiply
    Sa,
    /// Negate
    Na,
    /// Min
    Mi,
    /// Max
    Me,
    /// Reciprocal
    Ra,
    /// Magnitude
    Va,
    /// Sqrt
    La,
    /// Derivative
    We,
    /// Sine
    Wa,
    /// Index
    Ka,

    // Controls
    /// Horizontal slider
    Le,
    /// Vertical slider
    Li,
    /// X target
    Pe,
    /// Y target
    Pi,
    /// Activation 1
    Ve,
    /// Activation 2
    Vi,

    // Combinators
    /// Drop
    No,
    /// Duplicate
    Mo,
    /// Swap
    Ru,
    /// Over
    Vo,
}

impl Word {
    pub fn function(&self) -> Function {
        use Word::*;
        match self {
            To => Nullary::Zero.into(),
            Ti => Nullary::One.into(),
            Tu => Nullary::Two.into(),
            Ta => Nullary::Five.into(),
            Te => Nullary::Ten.into(),
            Ke => Nullary::OneX.into(),
            Ki => Nullary::OneY.into(),
            Ku => Variable::Vector.into(),
            Se => Nullary::X.into(),
            Si => Nullary::Y.into(),
            Su => Variable::Scalar.into(),
            Wi => ScalarInputFieldKind::Elevation.into(),
            Ro => ScalarInputFieldKind::Density.into(),
            Lu => ScalarInputFieldKind::Temperature.into(),
            Ko => ScalarInputFieldKind::Disorder.into(),
            Re => ScalarInputFieldKind::Memory.into(),
            Vu => VectorOutputFieldKind::Gravity.into(),
            Wu => VectorOutputFieldKind::Force.into(),
            Lo => ScalarOutputFieldKind::Heat.into(),
            Mu => ScalarOutputFieldKind::Order.into(),
            Nu => ScalarOutputFieldKind::Anchor.into(),
            Ri => VectorOutputFieldKind::Write.into(),
            Ma => HomoBinOp::Add.into(),
            Sa => HeteroBinOp::Mul.into(),
            Na => MathUnOp::Neg.into(),
            Mi => HomoBinOp::Min.into(),
            Me => HomoBinOp::Max.into(),
            Va => ToScalarOp::Magnitude.into(),
            Ra => ScalarUnOp::Reciprocal.into(),
            La => ScalarUnOp::Sqrt.into(),
            We => ScalarUnVectorOp::Derivative.into(),
            Wa => ScalarUnOp::Sin.into(),
            Ka => BinOp::Index.into(),
            No => Combinator1::Drop.into(),
            Mo => Combinator1::Duplicate.into(),
            Ru => Combinator2::Swap.into(),
            Vo => Combinator2::Over.into(),
            Le => ControlKind::XSlider.into(),
            Li => ControlKind::YSlider.into(),
            Pe => Nullary::TargetX.into(),
            Pi => Nullary::TargetY.into(),
            Ve => ControlKind::Activation1.into(),
            Vi => ControlKind::Activation2.into(),
        }
    }
    pub fn etchable(&self) -> bool {
        !matches!(self.function(), Function::Variable(_))
    }
    pub fn cost(&self) -> f32 {
        use Word::*;
        match self {
            To => 0.0,
            Ti => 1.0,
            Tu => 2.0,
            Ta => 5.0,
            Te => 10.0,
            Le => 2.0,
            Li => 2.0,
            Pe => 3.0,
            Pi => 3.0,
            No | Ru | Vo => 0.0,
            _ => 1.0,
        }
    }
    pub fn text_color(&self) -> Option<Color> {
        Some(match self.function() {
            Function::ReadField(_) => Color::rgb(0.7, 0.7, 1.0),
            Function::Nullary(
                Nullary::Zero | Nullary::One | Nullary::Two | Nullary::Five | Nullary::Ten,
            ) => Color::rgb(1.0, 0.7, 0.3),
            Function::Nullary(Nullary::ZeroVector | Nullary::OneX | Nullary::OneY) => {
                Color::rgb(0.5, 1.0, 1.0)
            }
            Function::Nullary(Nullary::X | Nullary::Y) => Color::rgb(1.0, 0.2, 0.5),
            Function::Nullary(Nullary::TargetX | Nullary::TargetY) | Function::Control(_) => {
                Color::rgb(1.0, 1.0, 0.3)
            }
            Function::Un(_) => Color::rgb(0.4, 1.0, 0.5),
            Function::Bin(_) => Color::rgb(1.0, 0.5, 1.0),
            Function::Variable(_) => Color::rgb(1.0, 0.7, 0.7),
            _ => return None,
        })
    }
}

struct Genotype {
    words: Vec<Word>,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Phenotype {
    grid: Vec<Vec<Word>>,
}

const MAX_ROWS: usize = 5;

impl Genotype {
    fn combine(&self, other: &Self, mut rng: impl Rng) -> Self {
        // Taken from https://www.reddit.com/r/KeyboardLayouts/comments/zcw0m5/comment/iz14ds4
        let left = &self.words;
        let right = &other.words;
        let index = rng.gen_range(0..left.len());
        let mut word_loop = vec![left[index], right[index]];
        loop {
            let index_in_left = self
                .words
                .iter()
                .position(|word| word == word_loop.last().unwrap())
                .unwrap();
            let word_in_right = right[index_in_left];
            if word_loop.contains(&word_in_right) {
                break;
            }
            word_loop.push(word_in_right);
        }
        let mut words = (0..left.len())
            .map(|i| {
                if word_loop.contains(&left[i]) {
                    left[i]
                } else {
                    right[i]
                }
            })
            .collect_vec();
        while rng.gen_bool(0.1) {
            let a = rng.gen_range(0..words.len());
            let b = rng.gen_range(0..words.len());
            words.swap(a, b);
        }
        Genotype { words }
    }
    fn arrange(&self) -> Phenotype {
        let max_cols = (cardinality::<Word>() as f32 / MAX_ROWS as f32).ceil() as usize;
        let mut grid: Vec<Vec<Word>> = Vec::new();
        let mut row = 0;
        let mut i = 0;
        loop {
            if i == self.words.len() {
                break;
            }
            if row < grid.len() {
                if grid[row].len() == max_cols {
                    row += 1;
                    continue;
                }
                grid[row].push(self.words[i]);
                i += 1;
                row += 1;
            } else if grid.len() < MAX_ROWS {
                grid.push(vec![self.words[i]]);
                i += 1;
                row = 0;
            } else {
                row = 0;
            }
        }
        grid.reverse();
        Phenotype { grid }
    }
}

use Word::*;
static REFERENCE_SPELLS: &[&[Word]] = &[
    &[Wi, Na, Ma, Ki, Sa, Vu],
    &[Te, Ki, Sa, Vu],
    &[Ti, Pe, Mo, Sa, Pi, Mo, Sa, Ma, Ma, Ra],
    &[Le, Ke, Sa, Vu],
    &[Le, Ke, Sa, Wu],
    &[Te, Li, Ma, Wi, Na, Ma, Ki, Sa, Vu],
    &[Ve, Sa],
    &[Vi, Sa],
    &[Mo, We, Sa],
    &[Te, Ta, Ma],
    &[Ta, Tu, Ma],
    &[Ta, Ti, Ma],
    &[Tu, Tu, Ma],
    &[Tu, Ti, Ma],
    &[Se, Pe, Na, Ma],
    &[Si, Pi, Na, Ma],
    &[Se, Mo, Sa],
    &[Si, Mo, Sa],
    &[Se, Ke],
    &[Si, Ki],
    &[Sa, Lo],
    &[Sa, Mu],
    &[Sa, Nu],
    &[To, Mi],
    &[To, Me],
    &[Ko, Sa],
    &[Ma, Ka],
    &[Ve, Mu],
    &[Vi, Mu],
    &[Re, Ka],
];
static GROUPS: &[&[Word]] = &[
    &[To, Ti, Tu, Ta, Te],
    &[Se, Si],
    &[Ke, Ki],
    &[Pe, Pi],
    &[Su, Ku],
    &[Le, Li],
    &[Vu, Wu],
    &[Ru, Vo],
    &[Mi, Me],
    &[Re, Ri],
    &[Ve, Vi],
];

impl Phenotype {
    fn word_index(&self, word: Word) -> [usize; 2] {
        for (i, row) in self.grid.iter().enumerate() {
            for (j, grid_word) in row.iter().enumerate() {
                if &word == grid_word {
                    return [i, j];
                }
            }
        }
        unreachable!()
    }
    fn fitness(&self) -> usize {
        let mut sum = 0.0;
        // Optimize for groups
        for group in GROUPS {
            for (&a, &b) in group.iter().tuple_windows() {
                let [ai, aj] = self.word_index(a);
                let [bi, bj] = self.word_index(b);
                let mut dist =
                    ((ai as f32 - bi as f32).powi(2) + (aj as f32 - bj as f32).powi(2)).sqrt();
                if ai > bi || aj > bj {
                    dist += 1.0;
                }
                sum += dist * 3.0;
            }
        }
        // Optimize for common spells
        for spell in REFERENCE_SPELLS {
            for (&a, &b) in spell.iter().tuple_windows() {
                let [ai, aj] = self.word_index(a);
                let [bi, bj] = self.word_index(b);
                let dist =
                    ((ai as f32 - bi as f32).powi(2) + (aj as f32 - bj as f32).powi(2)).sqrt();
                sum += dist;
            }
        }
        // Try to put numbers at the top
        for &number_word in &[To, Ti, Tu, Ta, Te] {
            let [i, _] = self.word_index(number_word);
            sum += (i * 3) as f32;
        }
        (sum * 1e6) as usize
    }
}

pub static WORD_GRID: Lazy<Vec<Vec<Word>>> = Lazy::new(|| {
    let path = resources_path().join("word_grid.yaml");
    if !env::args().any(|arg| arg == "regen_grid") {
        if let Some(grid) = fs::read_to_string(&path)
            .ok()
            .and_then(|yaml| serde_yaml::from_str(&yaml).ok())
        {
            return grid;
        }
    }

    let mut population = Vec::new();
    let mut rng = SmallRng::seed_from_u64(0);
    for _ in 0..100_000 {
        let mut words = all::<Word>().collect_vec();
        words.shuffle(&mut rng);
        population.push(Genotype { words });
    }
    for _ in 0..100 {
        population.par_sort_by_cached_key(|genotype| genotype.arrange().fitness());

        let fittest = &population[0];
        let fittest_fitness = fittest.arrange().fitness();
        println!("Fittest: {}", fittest_fitness);

        let best_half = &population[..population.len() / 2];
        let mut new_population = Vec::new();
        for _ in 0..population.len() {
            let a = best_half.choose(&mut rng).unwrap();
            let b = best_half.choose(&mut rng).unwrap();
            new_population.push(a.combine(b, &mut rng));
        }
        population = new_population;
    }
    population.par_sort_by_cached_key(|genotype| genotype.arrange().fitness());
    let final_grid = population.swap_remove(0).arrange().grid;

    for row in &final_grid {
        for word in row {
            print!("{word}{}", " ".repeat(6 - word.to_string().len()));
        }
        println!();
    }

    let _ = fs::write(path, serde_yaml::to_string(&final_grid).unwrap());

    final_grid
});
