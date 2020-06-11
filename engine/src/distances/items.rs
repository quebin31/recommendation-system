use num_traits::float::Float;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{AddAssign, Mul, Sub},
};

#[derive(Debug, Copy, Clone)]
pub enum Method {
    AdjCosine,
    SlopeOne,
}

pub fn adjusted_cosine_means<U, K, V>(vecs: &HashMap<U, HashMap<K, V>>) -> HashMap<U, V>
where
    U: Hash + Eq + Clone,
    K: Hash + Eq,
    V: Float + AddAssign,
{
    let mut means = HashMap::new();
    for (id, vec) in vecs {
        let mut mean = None;
        let mut n = 0;

        for x in vec.values() {
            *mean.get_or_insert_with(V::zero) += *x;
            n += 1;
        }

        if let Some(mean) = mean {
            let mean = mean / V::from(n).unwrap();
            means.insert(id.to_owned(), mean);
        }
    }

    means
}

pub fn fast_adjusted_cosine<U, K, V>(
    means: &HashMap<U, V>,
    vecs: &HashMap<U, HashMap<K, V>>,
    users_a: &HashSet<U>,
    users_b: &HashSet<U>,
    a: &K,
    b: &K,
) -> Option<V>
where
    U: Hash + Eq,
    K: Hash + Eq,
    V: Float + AddAssign + Sub + Mul,
{
    let mut cov = None;
    let mut dev_a = None;
    let mut dev_b = None;

    for common_user in users_a.intersection(users_b) {
        if vecs.get(common_user).is_none() {
            continue;
        }

        match (
            vecs[common_user].get(a),
            vecs[common_user].get(b),
            means.get(common_user),
        ) {
            (Some(val_a), Some(val_b), Some(mean)) => {
                *cov.get_or_insert_with(V::zero) += (*val_a - *mean) * (*val_b - *mean);
                *dev_a.get_or_insert_with(V::zero) += (*val_a - *mean).powi(2);
                *dev_b.get_or_insert_with(V::zero) += (*val_b - *mean).powi(2);
            }
            _ => continue,
        }
    }

    let num = cov?;
    let dem = dev_a?.sqrt() * dev_b?.sqrt();

    let res = num / dem;
    if res.is_nan() || res.is_infinite() {
        None
    } else {
        Some(res)
    }
}

pub fn slow_adjusted_cosine<U, K, V>(vecs: &HashMap<U, HashMap<K, V>>, a: &K, b: &K) -> Option<V>
where
    U: Hash + Eq + Clone,
    K: Hash + Eq,
    V: Float + AddAssign + Sub + Mul,
{
    let means = adjusted_cosine_means(vecs);
    fast_adjusted_cosine(&means, vecs, a, b)
}
