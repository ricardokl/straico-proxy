use serde::Serialize;
use std::borrow::Cow;

/// A request structure for generating text completions.
///
/// This struct represents a request to generate text completions with configurable parameters
/// such as model selection, input prompt, and various optional settings. It is designed to be
/// constructed using the builder pattern via `CompletionRequest::new()`.
///
/// # Fields
/// * `models` - The language model(s) to use for generating completions
/// * `message` - The input prompt text to generate completions from
/// * `file_urls` - Optional list of file URLs to provide as context
/// * `youtube_urls` - Optional list of YouTube URLs to provide as context
/// * `display_transcripts` - Optional flag to control transcript display
/// * `temperature` - Optional parameter controlling randomness in generation (0.0 to 1.0)
/// * `max_tokens` - Optional maximum number of tokens to generate
#[derive(Serialize)]
pub struct CompletionRequest<'a> {
    /// The language model(s) to use for generating completions, accepts up to 4 models
    models: RequestModels<'a>,
    /// The input prompt text to generate completions from
    message: Prompt<'a>,
    /// Optional list of file URLs to provide as context
    #[serde(skip_serializing_if = "Option::is_none")]
    file_urls: Option<Vec<&'a str>>,
    /// Optional list of YouTube URLs to provide as context
    #[serde(skip_serializing_if = "Option::is_none")]
    youtube_urls: Option<Vec<&'a str>>,
    /// Optional flag to control transcript display
    #[serde(skip_serializing_if = "Option::is_none")]
    display_transcripts: Option<bool>,
    /// Optional parameter controlling randomness in generation (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Optional maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// A newtype wrapper around `Cow<'a, str>` representing a prompt message for a completion request.
///
/// This struct encapsulates the actual text content of the prompt that will be used to generate
/// completions. It can hold either borrowed or owned string data through the `Cow` type.
#[derive(Serialize, Clone)]
pub struct Prompt<'a>(Cow<'a, str>);

impl<'a> From<Cow<'a, str>> for Prompt<'a> {
    /// Converts from `Cow<'a, str>` into a `Prompt<'a>`.
    ///
    /// This implementation allows us to create a `Prompt` from either a borrowed
    /// or owned string through the `Cow` type.
    ///
    /// # Arguments
    /// * `value` - A `Cow` containing either a borrowed or owned string
    ///
    /// # Returns
    /// A new `Prompt` wrapping the provided `Cow` value
    fn from(value: Cow<'a, str>) -> Self {
        Prompt(value)
    }
}

impl<'a> From<&'a str> for Prompt<'a> {
    /// Converts a string reference into a `Prompt<'a>`.
    ///
    /// This implementation allows creating a `Prompt` directly from a string reference,
    /// by converting it to a borrowed `Cow` internally.
    ///
    /// # Arguments
    /// * `value` - A string reference to convert into a prompt
    ///
    /// # Returns
    /// A new `Prompt` containing the provided string reference
    fn from(value: &'a str) -> Self {
        Prompt(Cow::Borrowed(value))
    }
}

impl AsRef<str> for Prompt<'_> {
    /// Implements the `AsRef<str>` trait for `Prompt`, allowing borrowing of the underlying string.
    ///
    /// This implementation provides a way to get a string slice reference from a `Prompt` instance,
    /// making it easier to use `Prompt` values in contexts that expect string references.
    ///
    /// # Returns
    /// A string slice referencing the underlying prompt text
    fn as_ref(&self) -> &str {
        let Prompt(x) = self;
        x
    }
}

/// A tuple struct representing the list of language models to use for completion generation.
///
/// This struct can hold up to four optional model identifiers, where each model is represented
/// as a `Cow<str>` that can contain either borrowed or owned string data. The struct uses
/// `serde`'s `skip_serializing_if` to omit any None values during serialization.
///
/// The four slots allow for requesting completions from multiple models in parallel, though
/// not all slots need to be filled. Typically only the first slot is used with a single model.
#[derive(Serialize)]
pub struct RequestModels<'a>(
    #[serde(skip_serializing_if = "Option::is_none")] Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")] Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")] Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")] Option<Cow<'a, str>>,
);

impl Default for RequestModels<'_> {
    /// Provides a default configuration for the language model request.
    ///
    /// The default configuration uses only the GPT-3.5 Turbo model (version 0125),
    /// with no additional model slots allocated. This provides a sensible starting
    /// point for basic completion requests while still allowing for customization
    /// if needed.
    ///
    /// # Returns
    /// A new `RequestModels` instance configured with the default GPT-3.5 Turbo model
    fn default() -> Self {
        Self(
            Some(Cow::Borrowed("openai/gpt-3.5-turbo-0125")),
            None,
            None,
            None,
        )
    }
}

impl<'a, const N: usize> From<[&'a str; N]> for RequestModels<'a>
where
    [(); N]: Max4,
{
    /// Converts an array of string references into a `RequestModels` instance.
    ///
    /// This implementation allows creating a `RequestModels` from an array of string references
    /// with length N (where N ≤ 4), converting each string reference into a `Cow::Borrowed`
    /// option. Unused slots are set to `None`.
    ///
    /// # Arguments
    /// * `arr` - An array of string references of length N (1 to 4)
    ///
    /// # Returns
    /// A new `RequestModels` instance containing up to 4 borrowed string references
    fn from(arr: [&'a str; N]) -> Self {
        let [a, b, c, d] = std::array::from_fn(|i| arr.get(i).map(|x| Cow::Borrowed(*x)));
        Self(a, b, c, d)
    }
}

trait Max4 {}
impl Max4 for [(); 1] {}
impl Max4 for [(); 2] {}
impl Max4 for [(); 3] {}
impl Max4 for [(); 4] {}

impl<'a> From<&'a str> for RequestModels<'a> {
    /// Converts a string reference into a `RequestModels` instance.
    ///
    /// This implementation creates a single-model `RequestModels` by wrapping the provided
    /// string reference in a single-element array and converting it using the array
    /// implementation of `From`.
    ///
    /// # Arguments
    /// * `value` - A string reference representing the model to use
    ///
    /// # Returns
    /// A new `RequestModels` instance containing a single borrowed string reference
    fn from(value: &'a str) -> Self {
        [value; 1].into()
    }
}

impl<'a> From<Cow<'a, str>> for RequestModels<'a> {
    /// Converts a `Cow<str>` into a `RequestModels` instance.
    ///
    /// This implementation creates a single-model `RequestModels` using the provided
    /// Cow string value in the first slot, with remaining slots set to None.
    ///
    /// # Arguments
    /// * `value` - A `Cow` string value representing the model to use
    ///
    /// # Returns
    /// A new `RequestModels` instance containing a single model specification
    fn from(value: Cow<'a, str>) -> Self {
        RequestModels(Some(value), None, None, None)
    }
}

/// A placeholder struct indicating that models have not been set in the builder.
///
/// This struct is used as a type parameter in `CompletionRequestBuilder` to track
/// whether the required models have been configured during the building process.
/// It implements `Default` to support builder initialization.
#[derive(Default)]
pub struct ModelsNotSet;

/// A placeholder struct indicating that a message has not been set in the builder.
///
/// This struct is used as a type parameter in `CompletionRequestBuilder` to track
/// whether the required message has been configured during the building process.
/// It implements `Default` to support builder initialization.
#[derive(Default)]
pub struct MessageNotSet;

/// Type alias representing a message that has been set in the builder.
///
/// This alias maps to `Prompt<'a>` and is used as a type parameter to indicate
/// that the message requirement has been satisfied.
type MessageSet<'a> = Prompt<'a>;

/// Type alias representing models that have been set in the builder.
///
/// This alias maps to `RequestModels<'a>` and is used as a type parameter to indicate
/// that the models requirement has been satisfied.
type ModelsSet<'a> = RequestModels<'a>;

/// A builder type for constructing `CompletionRequest` instances.
///
/// This struct uses generic type parameters T and K to track the status of required
/// fields (models and message) during the building process. The type parameters
/// will be either `ModelsNotSet`/`MessageNotSet` or `ModelsSet`/`MessageSet`
/// depending on whether those required fields have been configured.
///
/// # Type Parameters
/// * `'a` - The lifetime of string references used in the request
/// * `T` - Type indicating whether models have been set (ModelsNotSet or ModelsSet)
/// * `K` - Type indicating whether message has been set (MessageNotSet or MessageSet)
///
/// # Fields
/// * `models` - The language model(s) configuration
/// * `message` - The prompt message for completion
/// * `file_urls` - Optional list of file URLs to provide context
/// * `youtube_urls` - Optional list of YouTube URLs to provide context
/// * `display_transcripts` - Optional flag to control transcript display
/// * `temperature` - Optional parameter for generation randomness (0.0 to 2.0)
/// * `max_tokens` - Optional maximum number of tokens to generate
#[derive(Default)]
pub struct CompletionRequestBuilder<'a, T, K> {
    models: T,
    message: K,
    file_urls: Option<Vec<&'a str>>,
    youtube_urls: Option<Vec<&'a str>>,
    display_transcripts: Option<bool>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl<'a> CompletionRequest<'a> {
    /// Creates a new `CompletionRequestBuilder` with default values.
    ///
    /// This is the starting point for constructing a `CompletionRequest` using the builder pattern.
    /// The builder starts with no models or message set, which must be provided using the `models()`
    /// and `message()` methods before building.
    ///
    /// # Returns
    /// A `CompletionRequestBuilder` with default values and no models or message set.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> CompletionRequestBuilder<'a, ModelsNotSet, MessageNotSet> {
        CompletionRequestBuilder::default()
    }
}

impl<'a, T> CompletionRequestBuilder<'a, ModelsNotSet, T> {
    /// Sets the models for the completion request.
    ///
    /// Takes any type that can be converted into `RequestModels`, such as:
    /// - A single model string (&str)
    /// - An array of model strings ([&str; N] where N <= 4)
    /// - A `Cow<str>` containing a model string
    ///
    /// # Arguments
    /// * `models` - The model or models to use for completion
    ///
    /// # Returns
    /// A new `CompletionRequestBuilder` with the models set
    pub fn models<M>(self, models: M) -> CompletionRequestBuilder<'a, ModelsSet<'a>, T>
    where
        M: Into<RequestModels<'a>>,
    {
        CompletionRequestBuilder {
            models: models.into(),
            file_urls: self.file_urls,
            youtube_urls: self.youtube_urls,
            display_transcripts: self.display_transcripts,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            message: self.message,
        }
    }
}

impl<'a, T> CompletionRequestBuilder<'a, T, MessageNotSet> {
    /// Sets the message for the completion request.
    ///
    /// Takes any type that can be converted into `Prompt`, such as a string reference
    /// or `Cow<str>`.
    ///
    /// # Arguments
    /// * `message` - The message prompt to send for completion
    ///
    /// # Returns
    /// A new `CompletionRequestBuilder` with the message set
    pub fn message<M>(self, message: M) -> CompletionRequestBuilder<'a, T, MessageSet<'a>>
    where
        M: Into<Prompt<'a>>,
    {
        CompletionRequestBuilder {
            models: self.models,
            message: message.into(),
            file_urls: self.file_urls,
            youtube_urls: self.youtube_urls,
            display_transcripts: self.display_transcripts,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

impl<'a, T, K> CompletionRequestBuilder<'a, T, K> {
    /// Sets the file URLs for the completion request.
    ///
    /// # Arguments
    /// * `file_urls` - A slice of string references containing file URLs to include
    ///
    /// # Returns
    /// The builder with file URLs set
    pub fn file_urls(mut self, file_urls: &[&'a str]) -> Self {
        let _ = self.file_urls.insert(file_urls.into());
        self
    }

    /// Sets the YouTube URLs for the completion request.
    ///
    /// # Arguments
    /// * `youtube_urls` - A slice of string references containing YouTube URLs to include
    ///
    /// # Returns
    /// The builder with YouTube URLs set
    pub fn youtube_urls(mut self, youtube_urls: &[&'a str]) -> Self {
        let _ = self.youtube_urls.insert(youtube_urls.into());
        self
    }

    /// Sets whether to display transcripts in the completion request.
    ///
    /// # Arguments
    /// * `display_transcripts` - Boolean indicating whether to display transcripts
    ///
    /// # Returns
    /// The builder with display_transcripts preference set
    pub fn display_transcripts(mut self, display_transcripts: bool) -> Self {
        let _ = self.display_transcripts.insert(display_transcripts);
        self
    }

    /// Sets the temperature parameter for the completion request.
    ///
    /// # Arguments
    /// * `temperature` - A float value controlling randomness in the response
    ///
    /// # Returns
    /// The builder with temperature set
    pub fn temperature(mut self, temperature: f32) -> Self {
        let _ = self.temperature.insert(temperature);
        self
    }

    /// Sets the maximum number of tokens for the completion request.
    ///
    /// # Arguments
    /// * `max_tokens` - The maximum number of tokens to generate
    ///
    /// # Returns
    /// The builder with max_tokens set
    pub fn max_tokens(mut self, max_tokens: u32) -> CompletionRequestBuilder<'a, T, K> {
        let _ = self.max_tokens.insert(max_tokens);
        //self.max_tokens = Some(max_tokens);
        self
    }
}

impl<'a> CompletionRequestBuilder<'a, ModelsSet<'a>, MessageSet<'a>> {
    /// Builds the final `CompletionRequest` from the builder.
    ///
    /// This method can only be called once both the models and message have been set through
    /// the builder pattern. It consumes the builder and returns a fully constructed
    /// `CompletionRequest` with all configured options.
    ///
    /// # Returns
    /// A new `CompletionRequest` instance with all the settings specified in the builder.
    pub fn build(self) -> CompletionRequest<'a> {
        CompletionRequest {
            models: self.models,
            message: self.message,
            file_urls: self.file_urls,
            youtube_urls: self.youtube_urls,
            display_transcripts: self.display_transcripts,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

impl<'a> CompletionRequest<'a> {
    /// Returns the maximum number of tokens configured for this completion request.
    ///
    /// # Returns
    /// An `Option` containing a reference to the maximum tokens limit if set,
    /// or `None` if no limit was configured.
    pub fn get_max_tokens(&self) -> Option<&u32> {
        self.max_tokens.as_ref()
    }

    /// Returns the temperature setting for this completion request.
    ///
    /// Temperature controls the randomness of the generated responses, where higher values
    /// (e.g., 0.8) lead to more random outputs and lower values (e.g., 0.2) make the responses
    /// more focused and deterministic.
    ///
    /// # Returns
    /// An `Option` containing a reference to the temperature value if set,
    /// or `None` if no temperature was configured.
    pub fn get_temperature(&self) -> Option<&f32> {
        self.temperature.as_ref()
    }

    /// Returns whether transcript display is enabled for this completion request.
    ///
    /// # Returns
    /// An `Option` containing a reference to the display transcript setting if set,
    /// or `None` if the setting was not configured.
    pub fn get_display_transcripts(&self) -> Option<&bool> {
        self.display_transcripts.as_ref()
    }

    /// Returns the list of file URLs associated with this completion request.
    ///
    /// # Returns
    /// An `Option` containing a reference to the vector of file URLs if any were set,
    /// or `None` if no file URLs were configured.
    pub fn get_file_urls(&self) -> Option<&Vec<&'a str>> {
        self.file_urls.as_ref()
    }

    /// Returns the list of YouTube URLs associated with this completion request.
    ///
    /// # Returns
    /// An `Option` containing a reference to the vector of YouTube URLs if any were set,
    /// or `None` if no YouTube URLs were configured.
    pub fn get_youtube_urls(&self) -> Option<&Vec<&'a str>> {
        self.youtube_urls.as_ref()
    }
}
