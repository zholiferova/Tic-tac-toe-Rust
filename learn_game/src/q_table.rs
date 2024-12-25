use crate::board::Board;
use crate::config::{DISCOUNT_RATE, LEARNING_RATE};
use chrono::offset::Local;
use itertools::Itertools;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{prelude::*, BufReader};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::{fmt, fs::File};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moves {
    #[serde(serialize_with = "serialize_moves")]
    #[serde(deserialize_with = "deserialize_moves")]
    pub moves: HashMap<(usize, usize), f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QTable {
    qtable: HashMap<String, Moves>,
}

impl Deref for Moves {
    type Target = HashMap<(usize, usize), f32>;
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.moves
    }
}
impl DerefMut for Moves {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.moves
    }
}

impl Deref for QTable {
    type Target = HashMap<String, Moves>;
    fn deref(&self) -> &<Self as Deref>::Target {
        &self.qtable
    }
}

impl DerefMut for QTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.qtable
    }
}

impl Moves {
    pub fn new(m: Vec<(usize, usize)>) -> Moves {
        let mut rng = thread_rng();
        let moves: HashMap<(usize, usize), f32> = m
            .into_iter()
            .map(|(x, y)| ((x, y), rng.gen_range(-0.15f32..0.15f32)))
            .collect();
        Moves { moves }
    }
    pub fn select_max_move(&self) -> &(usize, usize) {
        let mut rng = thread_rng();
        let max_moves = self
            .iter()
            .max_set_by(|((_, _), &value1), ((_, _), &value2)| value1.total_cmp(&value2))
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<&(usize, usize)>>();
        *max_moves.choose(&mut rng).unwrap()
    }
    pub fn select_min_move(&self) -> &(usize, usize) {
        let mut rng = thread_rng();
        let max_moves = self
            .iter()
            .min_set_by(|((_, _), &value1), ((_, _), &value2)| value1.total_cmp(&value2))
            .into_iter()
            .map(|(key, _)| key)
            .collect::<Vec<&(usize, usize)>>();
        *max_moves.choose(&mut rng).unwrap()
    }
}

impl From<Vec<(usize, usize)>> for Moves {
    fn from(value: Vec<(usize, usize)>) -> Self {
        let mut map: Moves = Moves {moves: HashMap::with_capacity(10)};
        for m in value {
            map.insert(m, -100.0);
        }
        map
    }
}

impl QTable {
    pub fn new() -> Self {
        QTable {
            qtable: HashMap::with_capacity(11000),
        }
    }
    pub fn max_move(&self, state_key: String) -> &(usize, usize) {
        self.get(&state_key).unwrap().select_max_move()
    }
    pub fn min_move(&self, state_key: String) -> &(usize, usize) {
        self.get(&state_key).unwrap().select_max_move()
    }
    pub fn update_q_table(
        &mut self,
        current_state_key: &str,
        next_state_key: &str,
        current_move: &(usize, usize),
        player: &str,
        reward: f32,
        game_over: bool,
    ) {
        let expected = if game_over {
            reward
        } else if player == "RLmin" {
            let max_value = self
                .get(next_state_key)
                .expect("The next state key should be present.")
                .values()
                .max_by(|&value1, &value2| value1.total_cmp(&value2))
                .unwrap();
            DISCOUNT_RATE.with_borrow(|drate| drate * max_value)
        } else {
            let min_value = self
                .get(next_state_key)
                .expect("The current key should be present.")
                .values()
                .min_by(|&value1, &value2| value1.total_cmp(&value2))
                .unwrap();
            DISCOUNT_RATE.with_borrow(|drate| drate * min_value)
        };
        self.get_mut(current_state_key)
            .unwrap()
            .entry(*current_move)
            .and_modify(|value| LEARNING_RATE.with_borrow(|lrate| *value *= 1.0 - lrate))
            .and_modify(|value| LEARNING_RATE.with_borrow(|lrate| *value += lrate * expected));
    }
}

fn serialize_moves<S>(
    moves: &HashMap<(usize, usize), f32>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(moves.len()))?;
    for (k, v) in moves {
        let key_str = format!("({}, {})", k.0, k.1);
        map.serialize_entry(&key_str, &v)?;
    }
    map.end()
}

fn deserialize_moves<'de, D>(deserializer: D) -> Result<HashMap<(usize, usize), f32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct MapVisitor {
        marker: PhantomData<fn() -> HashMap<(usize, usize), f32>>,
    }
    impl MapVisitor {
        fn new() -> Self {
            MapVisitor {
                marker: PhantomData,
            }
        }
    }
    impl<'de> Visitor<'de> for MapVisitor {
        type Value = HashMap<(usize, usize), f32>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("my moves hashmap")
        }
        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));
            while let Some((key, value)) = access.next_entry::<String, f32>()? {
                let k: (usize, usize) = key
                    .chars()
                    .filter(|r| r.is_digit(10))
                    .map(|r| r.to_digit(10).unwrap() as usize)
                    .collect_tuple()
                    .expect("Expected two elements");
                map.insert(k, value);
            }
            Ok(map)
        }
    }
    deserializer.deserialize_map(MapVisitor::new())
}

pub fn q_table_to_disk(path: &std::path::Path, q: &QTable) -> Result<(), anyhow::Error> {
    let dt = Local::now();
    let today = dt.date_naive();
    let filename = "qtable-".to_owned() + (&today.to_string()) + r#".pickle"#;
    let filename_json = "qtable-".to_owned() + &today.to_string() + r#".json"#;
    let q_json: PathBuf = [path, &Path::new(&filename_json)].iter().collect();
    let q_pickle: PathBuf = [path, &Path::new(&filename)].iter().collect();
    let mut file = File::create(&q_pickle)?;
    let mut file_json = File::create(&q_json)?;
    let data_json = serde_json::to_string(&q).unwrap();
    file_json.write_all(&data_json.as_bytes())?;
    serde_pickle::to_writer(&mut file, q, serde_pickle::SerOptions::new())?;
    Ok(())
}

pub fn q_table_from_disk_pickle(file: &std::path::Path) -> Result<QTable, anyhow::Error> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);
    let mut buf: Vec<u8> = vec![];
    reader.read_to_end(&mut buf).unwrap();
    let decoded: QTable = serde_pickle::from_slice(&buf, serde_pickle::DeOptions::new())?;
    Ok(decoded)
}

pub fn q_table_from_disk_json(file: &std::path::Path) -> Result<QTable, anyhow::Error> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);
    let mut buf: String = "".to_owned();
    reader.read_to_string(&mut buf).unwrap();
    let decoded: QTable = serde_json::from_str(&buf)?;
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_q_table_working() {
        let mut q = QTable::new();
        let mut test_board = Board::new();
        test_board.current_state.state[[1, 1]] = 'X';
        test_board.current_state.state[[2, 2]] = '0';
        test_board.current_state.draw();
        let q_key = test_board.current_state.to_state_key();
        println!("q_key is {}", q_key);
        let t = q
            .entry(q_key)
            .or_insert(Moves::new(test_board.current_state.available_moves()));
        println!("{t:?}");
        println!("{:?}", q.qtable);
        let q_key = test_board.current_state.to_state_key();
        test_board.current_state.state[[0, 0]] = 'X';
        let q_next_key = test_board.next_state.to_state_key();
        let t_next = q
            .entry(q_next_key)
            .or_insert(Moves::new(test_board.current_state.available_moves()));
        let q_next_key = test_board.next_state.to_state_key();
        println!("Before update, next state: {:?}", q.get(&q_next_key));
        println!("Before update: {:?}", q.get(&q_key));
        q.update_q_table(&q_key, &q_next_key, &(0, 0), &"RLmax", 0.0, false);
        println!("After update: {:?}", q.get(&q_key));
        let m = QTable::max_move(&q, q_key);
        println!("{m:?}");
    }
    #[test]
    fn is_q_table_to_disk_working() {
        let mut q = QTable::new();
        let mut test_board = Board::new();
        test_board.current_state.state[[1, 1]] = 'X';
        test_board.current_state.state[[2, 2]] = '0';
        let q_key = test_board.current_state.to_state_key();
        let t = q
            .entry(q_key)
            .or_insert(Moves::new(test_board.current_state.available_moves()));
        let path = Path::new("../q_table_archive/");
        let q_saved = q_table_to_disk(&path, &q);
    }
    #[test]
    fn is_moves_working() {
        let moves: Vec<(usize, usize)> = vec![(0,2), (1,1), (3,3)];
        let moves_values: Moves = Moves::from(moves);
        println!("{:?}", moves_values);
    }
}
