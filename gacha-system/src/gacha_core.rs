use gdnative::{export::Export, prelude::*};
use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};
use std::{collections::HashMap, ops::Range};

use crate::error::{GachaError, Result};

#[derive(ToVariant, FromVariant, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    SSR,
    SR,
    R,
    N,
}

impl ToVariantEq for Rarity {}

#[derive(Debug, ToVariant, FromVariant, Clone)]
pub struct GachaItem {
    pub name: String,
    pub rarity: Rarity,
}

impl Export for GachaItem {
    type Hint = ();
    fn export_info(_hint: Option<Self::Hint>) -> ExportInfo {
        ExportInfo::new(VariantType::Dictionary)
    }
}

#[derive(NativeClass, Debug, Default)]
#[inherit(Node)]
pub struct GachaSystem {
    #[property]
    chances: u32,
    #[property]
    pity: u32,
    #[property]
    hard_pity: u32,
    /// Accumulated ammount of pulls before hitting any pity.
    _pity_accu: u32,
    /// Accumulated ammount of pulls before hitting hard pity.
    _hard_pity_accu: u32,
    #[property]
    data: HashMap<Rarity, Vec<GachaItem>>,
    #[property]
    rarities: Vec<(Rarity, f64)>,
}

#[methods]
impl GachaSystem {
    fn new(_owner: &Node) -> Self {
        let default_rarities = vec![
            (Rarity::SSR, 0.05),
            (Rarity::SR, 0.2),
            (Rarity::R, 0.4),
            (Rarity::N, 0.35),
        ];
        GachaSystem {
            pity: 10,
            hard_pity: 50,
            rarities: default_rarities.into(),
            ..Default::default()
        }
    }

    #[method]
    fn _ready(&self) {
        godot_print!("rarities: {:?}", self.rarities);
    }

    #[method]
    fn pull(&mut self, num: u32) -> Vec<GachaItem> {
        let mut result = vec![];
        let mut rng = thread_rng();
        let gen_limit: f64 = self.rarities.iter().map(|(_, ra)| ra).sum();
        let num_limit = num.min(self.chances);

        for _ in 0..num_limit {
            if let Some(pity_rarity) = self.get_pity() {
                result.push(self.gacha_by_rarity(pity_rarity, &mut rng).unwrap());
                continue;
            }
            // generate a random float within the limit
            let f = rng.gen_range(0.0..gen_limit);

            for (rarity, range) in rarity_range(&self.rarities) {
                if range.contains(&f) {
                    godot_print!("rolled: {f}, you got a: {:?} item", rarity);
                    // FIXME: change this to randomly draw item from poll later
                    result.push(self.gacha_by_rarity(rarity, &mut rng).unwrap());
                    break;
                }
            }
        }
        result
    }

    fn gacha_by_rarity(&mut self, rarity: Rarity, rng: &mut ThreadRng) -> Result<GachaItem> {
        let poll = self
            .data
            .get(&rarity)
            .ok_or_else(|| GachaError::InvalidRarity(format!("{rarity:?}")))?;
        let res = poll
            .choose(rng)
            .ok_or_else(|| GachaError::RarityWithNoData(format!("{rarity:?}")))?
            .clone();

        // only update counters when successfully pulled
        self.chances -= 1;
        match rarity {
            Rarity::SSR => {
                self._hard_pity_accu = 0;
                self._pity_accu = 0;
            }
            Rarity::SR => {
                self._hard_pity_accu += 1;
                self._pity_accu = 0;
            }
            _ => {
                self._hard_pity_accu += 1;
                self._pity_accu += 1;
            }
        }

        Ok(res)
    }

    /// Return the coresponding rarity if a pity was hit.
    fn get_pity(&mut self) -> Option<Rarity> {
        if self._hard_pity_accu + 1 == self.hard_pity {
            Some(Rarity::SSR)
        } else if self._pity_accu + 1 == self.pity {
            Some(Rarity::SR)
        } else {
            None
        }
    }
}

fn rarity_range(rarities: &[(Rarity, f64)]) -> Vec<(Rarity, Range<f64>)> {
    let mut hashmap = Vec::new();
    let mut sum = 0.0;
    for (rarity, rate) in rarities {
        let lo = sum;
        let hi = lo + rate;
        hashmap.push((*rarity, lo..hi));
        sum = hi;
    }
    hashmap
}

#[cfg(test)]
mod tests {
    use super::{rarity_range, GachaItem, GachaSystem, Range, Rarity};
    use lazy_static::lazy_static;
    use std::collections::HashMap;

    static RARITIES: &[(Rarity, f64)] = &[
        (Rarity::SSR, 0.05),
        (Rarity::N, 0.35),
        (Rarity::R, 0.4),
        (Rarity::SR, 0.2),
    ];

    lazy_static! {
        static ref DATA: HashMap<Rarity, Vec<GachaItem>> = HashMap::from([
            (Rarity::SSR, gacha_items(Rarity::SSR, 2)),
            (Rarity::SR, gacha_items(Rarity::SR, 3)),
            (Rarity::R, gacha_items(Rarity::R, 4)),
            (Rarity::N, gacha_items(Rarity::N, 3)),
        ]);
    }

    fn gacha_items(rarity: Rarity, num: u8) -> Vec<GachaItem> {
        let mut res = vec![];
        for i in 0..num {
            let name = format!("{rarity:?}-{i}");
            res.push(GachaItem { name, rarity });
        }
        res
    }

    #[test]
    fn ranges() {
        let precision_round = |x: f64, mul: f64| -> f64 { (x * mul).round() / mul };

        let actural: Vec<Range<f64>> = rarity_range(&RARITIES)
            .iter()
            .map(|(_, rg)| precision_round(rg.start, 100.0)..precision_round(rg.end, 100.0))
            .collect();
        let expected = vec![0.00..0.05, 0.05..0.40, 0.40..0.80, 0.80..1.00];
        assert_eq!(actural, expected);
    }

    #[test]
    fn pull() {
        let mut gacha = GachaSystem {
            chances: 11,
            rarities: RARITIES.to_owned(),
            data: DATA.clone(),
            ..Default::default()
        };
        let res = gacha.pull(1);
        println!("1 pull: {:?}", res);
        assert_eq!(res.len(), 1);

        let ten_poll_res = gacha.pull(10);
        println!("10 pull: {:?}", ten_poll_res);
        assert_eq!(ten_poll_res.len(), 10);
    }

    #[test]
    fn saturating_pull() {
        let mut gacha = GachaSystem {
            chances: 8,
            rarities: RARITIES.to_owned(),
            data: DATA.clone(),
            ..Default::default()
        };
        let res = gacha.pull(1000);
        println!("all pull: {:?}", res);
        assert_eq!(res.len(), 8);
    }

    #[test]
    fn pity_system() {
        let mut gacha = GachaSystem {
            chances: 100,
            rarities: vec![
                (Rarity::SSR, 0.00),
                (Rarity::SR, 0.00),
                (Rarity::R, 0.5),
                (Rarity::N, 0.5),
            ],
            pity: 10,
            hard_pity: 80,
            data: DATA.clone(),
            ..Default::default()
        };

        let _ = gacha.pull(9);
        let has_sr = gacha.pull(1);
        // Since the rate of pulling SR is 0, so the 10th pull will always be SR
        assert_eq!(has_sr.get(0).map(|gd| gd.rarity), Some(Rarity::SR));

        let _ = gacha.pull(69);
        let has_ssr = gacha.pull(1);
        // Since the rate of pulling SSR is 0, so the 80th pull will always be SSR
        assert_eq!(gacha.chances, 20);
        assert_eq!(has_ssr.get(0).map(|gd| gd.rarity), Some(Rarity::SSR));
    }
}
