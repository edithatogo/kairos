// WGSL Compute Shader: candidate_scoring.wgsl
// Workgroup size: 256
// Computes candidate evaluation metric reduction (W1 and KS accumulation)
// for a batch of parameter candidates against an empirical reference.

struct CandidateScoringParams {
    obs_count: u32,
    candidate_count: u32,
    weight_w1: f32,
    weight_ks: f32,
};

struct CandidateScoreResult {
    candidate_id: u32,
    w1_metric: f32,
    ks_metric: f32,
    composite_score: f32,
};

@group(0) @binding(0)
var<storage, read> reference_obs: array<f32>;

@group(0) @binding(1)
var<storage, read> candidate_preds: array<f32>;

@group(0) @binding(2)
var<storage, read_write> scores: array<CandidateScoreResult>;

@group(0) @binding(3)
var<uniform> params: CandidateScoringParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cand_idx = id.x;
    if (cand_idx >= params.candidate_count) {
        return;
    }

    let n = params.obs_count;
    if (n == 0u) {
        scores[cand_idx].candidate_id = cand_idx;
        scores[cand_idx].w1_metric = 0.0;
        scores[cand_idx].ks_metric = 0.0;
        scores[cand_idx].composite_score = 0.0;
        return;
    }

    let offset = cand_idx * n;
    var sum_abs_diff: f32 = 0.0;
    var max_rel_diff: f32 = 0.0;
    let inv_n = 1.0 / f32(n);

    // Accumulate W1 (L1 pointwise differences) and KS maximum empirical discrepancy
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let p = candidate_preds[offset + i];
        let r = reference_obs[i];

        let diff = abs(p - r);
        sum_abs_diff = sum_abs_diff + diff;

        let pt_ks = diff / (abs(r) + 1.0);
        if (pt_ks > max_rel_diff) {
            max_rel_diff = pt_ks;
        }
    }

    let w1 = sum_abs_diff * inv_n;
    let ks = min(max_rel_diff, 1.0);
    let composite = params.weight_w1 * w1 + params.weight_ks * ks;

    scores[cand_idx].candidate_id = cand_idx;
    scores[cand_idx].w1_metric = w1;
    scores[cand_idx].ks_metric = ks;
    scores[cand_idx].composite_score = composite;
}
