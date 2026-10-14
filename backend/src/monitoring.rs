use crate::models::{Inspection, CLASSES};
use serde_json::{json, Value};

pub fn summarize(rows: &[Inspection], reference: Option<&[f64]>) -> Value {
    let total = rows.len();
    let reviewed: Vec<_> = rows.iter().filter(|r| r.review.is_some()).collect();
    let corrections = reviewed
        .iter()
        .filter(|r| r.review.as_ref().unwrap().label != r.prediction.label)
        .count();
    let mut classes = [0usize; 6];
    let mut confidence = [0usize; 10];
    let mut latency: Vec<f64> = rows.iter().map(|r| r.prediction.inference_ms).collect();
    latency.sort_by(f64::total_cmp);
    for r in rows {
        if let Some(i) = CLASSES.iter().position(|c| *c == r.prediction.label) {
            classes[i] += 1;
        }
        confidence[((r.prediction.confidence * 10.0) as usize).min(9)] += 1;
    }
    let mean = if total == 0 {
        0.0
    } else {
        latency.iter().sum::<f64>() / total as f64
    };
    let p95 = latency
        .get(((total as f64 * 0.95).ceil() as usize).saturating_sub(1))
        .copied()
        .unwrap_or(0.0);
    // PSI is a heuristic for class-frequency shift, not proof of model degradation.
    let psi = reference.filter(|r| r.len() == 6 && total >= 30).map(|r| {
        r.iter()
            .zip(classes)
            .map(|(expected, count)| {
                let a = expected.max(1e-4);
                let b = (count as f64 / total as f64).max(1e-4);
                (b - a) * (b / a).ln()
            })
            .sum::<f64>()
    });
    json!({"total":total,"awaiting_review":total-reviewed.len(),"reviewed":reviewed.len(),
        "low_confidence":rows.iter().filter(|r| r.prediction.confidence < 0.7).count(),
        "class_counts":CLASSES.iter().zip(classes).map(|(c,n)| (c.to_string(), json!(n))).collect::<serde_json::Map<_,_>>(),
        "confidence_bins":confidence,"mean_latency_ms":mean,"p95_latency_ms":p95,
        "corrections":corrections,"reviewed_agreement":if reviewed.is_empty(){None}else{Some(1.0-corrections as f64/reviewed.len() as f64)},
        "psi":psi,"drift_warning":psi.is_some_and(|v| v > 0.2),
        "latency_trend": rows.iter().rev().take(50).map(|r| json!({"at":r.created_at,"ms":r.prediction.inference_ms})).collect::<Vec<_>>()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_is_not_accuracy() {
        assert_eq!(summarize(&[], None)["reviewed_agreement"], Value::Null);
    }
}
