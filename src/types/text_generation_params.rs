use serde::{Deserialize, Serialize};

use crate::pb;

/// Text generation parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextGenerationParams {
    // Leave most validation of parameters to downstream text generation servers
    /// Maximum number of new tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_new_tokens: Option<u32>,
    /// Minimum number of new tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_new_tokens: Option<u32>,
    /// Truncate to this many input tokens for generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate_input_tokens: Option<u32>,
    /// The high level decoding strategy for picking
    /// tokens during text generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoding_method: Option<String>,
    /// Number of highest probability vocabulary tokens to keep for top-k-filtering.
    /// Only applies for sampling mode. When decoding_strategy is set to sample,
    /// only the top_k most likely tokens are considered as candidates for the next generated token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Similar to top_k except the candidates to generate the next token are the
    /// most likely tokens with probabilities that add up to at least top_p.
    /// Also known as nucleus sampling. A value of 1.0 is equivalent to disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Local typicality measures how similar the conditional probability of
    /// predicting a target token next is to the expected conditional
    /// probability of predicting a random token next, given the partial text
    /// already generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typical_p: Option<f64>,
    /// A value used to modify the next-token probabilities in sampling mode.
    /// Values less than 1.0 sharpen the probability distribution, resulting in
    /// "less random" output. Values greater than 1.0 flatten the probability distribution,
    /// resulting in "more random" output. A value of 1.0 has no effect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Represents the penalty for penalizing tokens that have already been generated
    /// or belong to the context. The value 1.0 means that there is no penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    /// Time limit in milliseconds for text generation to complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_time: Option<f64>,
    /// Parameters to exponentially increase the likelihood of the text generation
    /// terminating once a specified number of tokens have been generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exponential_decay_length_penalty: Option<ExponentialDecayLengthPenalty>,
    /// One or more strings which will cause the text generation to stop if/when
    /// they are produced as part of the output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Random seed used for text generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// Whether or not to include input text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_input_text: Option<bool>,
    /// Whether or not to include input text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<bool>,
    /// Whether or not to include list of individual generated tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_tokens: Option<bool>,
    /// Whether or not to include logprob for each returned token
    /// Applicable only if generated_tokens == true and/or input_tokens == true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_logprobs: Option<bool>,
    /// Whether or not to include rank of each returned token
    /// Applicable only if generated_tokens == true and/or input_tokens == true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_ranks: Option<bool>,
    /// Whether or not to include stop sequence
    /// If not specified, default behavior depends on server setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_stop_sequence: Option<bool>,
}

impl From<TextGenerationParams> for pb::fmaas::Parameters {
    fn from(value: TextGenerationParams) -> Self {
        let decoding_method = value.decoding_method.unwrap_or("GREEDY".to_string());
        let method = pb::fmaas::DecodingMethod::from_str_name(&decoding_method).unwrap_or_default();
        let sampling = pb::fmaas::SamplingParameters {
            temperature: value.temperature.unwrap_or_default() as f32,
            top_k: value.top_k.unwrap_or_default(),
            top_p: value.top_p.unwrap_or_default() as f32,
            typical_p: value.typical_p.unwrap_or_default() as f32,
            seed: value.seed.map(|v| v as u64),
        };
        let stopping = pb::fmaas::StoppingCriteria {
            max_new_tokens: value.max_new_tokens.unwrap_or_default(),
            min_new_tokens: value.min_new_tokens.unwrap_or_default(),
            time_limit_millis: value.max_time.unwrap_or_default() as u32,
            stop_sequences: value.stop_sequences.unwrap_or_default(),
            include_stop_sequence: value.include_stop_sequence,
        };
        let response = pb::fmaas::ResponseOptions {
            input_text: value.preserve_input_text.unwrap_or_default(),
            generated_tokens: value.generated_tokens.unwrap_or_default(),
            input_tokens: value.input_tokens.unwrap_or_default(),
            token_logprobs: value.token_logprobs.unwrap_or_default(),
            token_ranks: value.token_ranks.unwrap_or_default(),
            top_n_tokens: 0, // missing?
        };
        let decoding = pb::fmaas::DecodingParameters {
            repetition_penalty: value.repetition_penalty.unwrap_or_default() as f32,
            length_penalty: value.exponential_decay_length_penalty.map(Into::into),
        };
        let truncate_input_tokens = value.truncate_input_tokens.unwrap_or_default();
        Self {
            method: method as i32,
            sampling: Some(sampling),
            stopping: Some(stopping),
            response: Some(response),
            decoding: Some(decoding),
            truncate_input_tokens,
            beam: None, // missing?
        }
    }
}

/// Parameters to exponentially increase the likelihood of the text generation
/// terminating once a specified number of tokens have been generated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExponentialDecayLengthPenalty {
    /// Start the decay after this number of tokens have been generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,
    /// Factor of exponential decay
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decay_factor: Option<f64>,
}

impl From<ExponentialDecayLengthPenalty> for pb::fmaas::decoding_parameters::LengthPenalty {
    fn from(value: ExponentialDecayLengthPenalty) -> Self {
        Self {
            start_index: value.start_index.unwrap_or_default(),
            decay_factor: value.decay_factor.unwrap_or_default() as f32,
        }
    }
}

impl From<ExponentialDecayLengthPenalty>
    for pb::caikit_data_model::caikit_nlp::ExponentialDecayLengthPenalty
{
    fn from(value: ExponentialDecayLengthPenalty) -> Self {
        Self {
            start_index: value.start_index.map(|v| v as i64).unwrap_or_default(),
            decay_factor: value.decay_factor.unwrap_or_default(),
        }
    }
}
