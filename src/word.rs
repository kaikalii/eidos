use std::{env, fs};

use derive_more::{Display, From};
use enum_iterator::{all, cardinality, Sequence};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rand::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{field::*, function::*, utils::resources_path};

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
    Seva,
    /// Y
    Sevi,
    /// Scalar variable
    Sevu,
    // Vectors
    /// Unit vector X
    Kova,
    /// Unit vector Y
    Kovi,
    /// Vector variable
    Kovu,
    // Inputs
    /// Elevation
    Le,
    /// Density
    Po,
    /// Light
    Lusa,
    /// Heat
    Selo,
    /// Magic
    Mesi,
    // Outputs
    /// Gravity
    Ke,
    /// Force
    Pe,
    /// Heat pressure
    Sela,
    // Operators
    /// Add
    Ma,
    /// Multiply
    Sa,
    /// Negate
    Na,
    /// Min
    Meki,
    /// Max
    Meka,
    /// Reciprocal
    Reso,
    /// Magnitude
    Solo,
    /// Sqrt
    Kuru,
    /// Derivative
    Riva,
    /// Sine
    Wava,
    // Controls
    /// Horizontal slider
    Sila,
    /// Vertical slider
    Vila,
    /// X target
    Pa,
    /// Y target
    Pi,
    /// Activation
    Veni,
    // Combinators
    /// Drop
    No,
    /// Duplicate
    Mo,
    /// Swap
    Revi,
    /// Over
    Rovo,
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
            Kova => Nullary::OneX.into(),
            Kovi => Nullary::OneY.into(),
            Kovu => Variable::Vector.into(),
            Seva => Nullary::X.into(),
            Sevi => Nullary::Y.into(),
            Sevu => Variable::Scalar.into(),
            Le => ScalarInputFieldKind::Elevation.into(),
            Po => ScalarInputFieldKind::Density.into(),
            Lusa => ScalarInputFieldKind::Light.into(),
            Selo => ScalarInputFieldKind::Heat.into(),
            Mesi => ScalarInputFieldKind::Magic.into(),
            Ke => VectorOutputFieldKind::Gravity.into(),
            Pe => VectorOutputFieldKind::Force.into(),
            Sela => ScalarOutputFieldKind::Heat.into(),
            Ma => HomoBinOp::Add.into(),
            Sa => HeteroBinOp::Mul.into(),
            Na => MathUnOp::Neg.into(),
            Meki => HomoBinOp::Min.into(),
            Meka => HomoBinOp::Max.into(),
            Solo => ToScalarOp::Magnitude.into(),
            Reso => ScalarUnOp::Reciprocal.into(),
            Kuru => ScalarUnOp::Sqrt.into(),
            Riva => ScalarUnVectorOp::Derivative.into(),
            Wava => ScalarUnOp::Sin.into(),
            No => Combinator1::Drop.into(),
            Mo => Combinator1::Duplicate.into(),
            Revi => Combinator2::Swap.into(),
            Rovo => Combinator2::Over.into(),
            Sila => ControlKind::XSlider.into(),
            Vila => ControlKind::YSlider.into(),
            Pa => Nullary::TargetX.into(),
            Pi => Nullary::TargetY.into(),
            Veni => ControlKind::Activation.into(),
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
            Sila => 2.0,
            Vila => 2.0,
            Pa => 3.0,
            Pi => 3.0,
            No | Revi | Rovo => 0.0,
            _ => 1.0,
        }
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
    &[Le, Na, Ma, Kovi, Sa, Ke],
    &[Te, Kovi, Sa, Ke],
    &[Ti, Pa, Mo, Sa, Pi, Mo, Sa, Ma, Ma, Reso],
    &[Sila, Kova, Sa, Ke],
    &[Sila, Kova, Sa, Pe],
    &[Te, Vila, Ma, Le, Na, Ma, Kovi, Sa, Ke],
    &[Veni, Sa],
    &[Mo, Riva, Sa],
    &[Te, Ta, Ma],
    &[Ta, Tu, Ma],
    &[Ta, Ti, Ma],
    &[Tu, Tu, Ma],
    &[Tu, Ti, Ma],
    &[Seva, Pa, Na, Ma],
    &[Sevi, Pi, Na, Ma],
    &[Seva, Mo, Sa],
    &[Sevi, Mo, Sa],
    &[Seva, Kova],
    &[Sevi, Kovi],
    &[Sa, Sela],
    &[To, Meki],
    &[To, Meka],
];
static GROUPS: &[&[Word]] = &[
    &[To, Ti, Tu, Ta, Te],
    &[Seva, Sevi],
    &[Kova, Kovi],
    &[Pa, Pi],
    &[Sevu, Kovu],
    &[Sila, Vila],
    &[Ke, Pe],
    &[Revi, Rovo],
    &[Meki, Meka],
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
        let mut sum = 0;
        // Optimize for groups
        for group in GROUPS {
            for (&a, &b) in group.iter().tuple_windows() {
                let [ai, aj] = self.word_index(a);
                let [bi, bj] = self.word_index(b);
                let mut dist = ai.abs_diff(bi) + aj.abs_diff(bj);
                if ai > bi {
                    dist += 1;
                }
                if aj > bj {
                    dist += 1;
                }
                sum += dist * 2;
            }
        }
        // Optimize for common spells
        for spell in REFERENCE_SPELLS {
            for (&a, &b) in spell.iter().tuple_windows() {
                let [ai, aj] = self.word_index(a);
                let [bi, bj] = self.word_index(b);
                let dist = ai.abs_diff(bi) + aj.abs_diff(bj);
                sum += dist;
            }
        }
        // Try to put numbers at the top
        for &number_word in &[To, Ti, Tu, Ta, Te] {
            let [i, _] = self.word_index(number_word);
            sum += i * 3;
        }
        sum
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
        println!("Fittest fitness: {}", fittest_fitness);

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
