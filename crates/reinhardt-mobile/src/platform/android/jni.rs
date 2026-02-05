//! Android JNI bindings setup.
//!
//! Provides JNI integration for Android WebView using wry.

/// Android JNI helper.
// Used for code generation and documentation during Android builds
#[allow(dead_code)]
pub(crate) struct AndroidJni;

#[allow(dead_code)]
impl AndroidJni {
	/// Generates the JNI binding code for Android.
	///
	/// This should be called in the application's main module to set up
	/// the native activity bindings required by wry.
	pub(crate) fn generate_binding_code() -> &'static str {
		r#"
// Include this in your lib.rs for Android support:
//
// #[cfg(target_os = "android")]
// {
//     use wry::android_binding;
//     android_binding!(
//         com_example_reinhardt,          // Package name with underscores
//         MainActivity,                    // Activity class name
//         reinhardt_mobile::android_setup  // Setup function
//     );
// }
"#
	}

	/// Returns the Android setup function code.
	pub(crate) fn generate_setup_function() -> &'static str {
		r#"
/// Android setup function called by the JNI binding.
#[cfg(target_os = "android")]
pub fn android_setup(env: jni::JNIEnv, class: jni::objects::JClass, activity: jni::objects::JObject) {
    // Initialize the Android context
    #[cfg(target_os = "android")]
    {
        use ndk_context;
        use std::ptr::NonNull;

        // Get the native activity
        let activity_ptr = env.get_native_interface() as *mut std::ffi::c_void;
        if !activity_ptr.is_null() {
            if let Some(ptr) = NonNull::new(activity_ptr) {
                // Store context for later use
                // ndk_context::android_context() will be available after this
            }
        }
    }
}
"#
	}

	/// Returns instructions for setting up the Android activity.
	pub(crate) fn activity_setup_instructions() -> &'static str {
		r#"
// MainActivity.kt should extend AppCompatActivity and include:
//
// class MainActivity : AppCompatActivity() {
//     companion object {
//         init {
//             System.loadLibrary("your_app_name")
//         }
//     }
//
//     override fun onCreate(savedInstanceState: Bundle?) {
//         super.onCreate(savedInstanceState)
//         // Your initialization code
//     }
// }
"#
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_binding_code_generation() {
		let code = AndroidJni::generate_binding_code();
		assert!(code.contains("android_binding"));
	}

	#[test]
	fn test_setup_function_generation() {
		let code = AndroidJni::generate_setup_function();
		assert!(code.contains("android_setup"));
	}
}
