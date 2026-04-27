//! Device detection based on User-Agent header.
//! Lightweight implementation without external crates for sub-millisecond detection.

/// Represents the type of device based on User-Agent analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceType {
    /// iOS devices (iPhone, iPad, iPod)
    IOS,
    /// Android devices (phones and tablets)
    Android,
    /// Desktop devices (Windows, macOS, Linux)
    Desktop,
    /// Unknown or unrecognized devices (bots, crawlers, etc.)
    Other,
}

impl DeviceType {
    /// Returns a string representation of the device type.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::IOS => "ios",
            DeviceType::Android => "android",
            DeviceType::Desktop => "desktop",
            DeviceType::Other => "other",
        }
    }
}

/// Detect the device type from a User-Agent string.
///
/// Detection order matters - more specific patterns are checked first.
/// This implementation prioritizes accuracy for the most common device types.
pub fn detect_device(user_agent: &str) -> DeviceType {
    let ua = user_agent.to_lowercase();

    // Check for iOS first (includes iPhone, iPad, iPod)
    // iPad in iPadOS 13+ may report as "Macintosh" with "Safari" but still has touch indicators
    if ua.contains("iphone")
        || ua.contains("ipad")
        || ua.contains("ipod")
        || (ua.contains("macintosh") && ua.contains("mobile"))
    {
        return DeviceType::IOS;
    }

    // Check for Android (but not Android TV or other variants)
    if ua.contains("android") && !ua.contains("android tv") && !ua.contains("androidtv") {
        return DeviceType::Android;
    }

    // Check for mobile indicators - if present but not iOS/Android, categorize as Other
    // This catches Windows Phone, BlackBerry, etc.
    if ua.contains("mobile") || ua.contains("phone") || ua.contains("mobi") {
        return DeviceType::Other;
    }

    // Desktop patterns (Windows, macOS, Linux)
    // These checks come after mobile to avoid misclassifying mobile devices
    if ua.contains("windows")
        || ua.contains("macintosh")
        || ua.contains("mac os")
        || ua.contains("linux")
        || ua.contains("x11")
        || ua.contains("x64")
        || ua.contains("x86_64")
        || ua.contains("wow64")
    {
        return DeviceType::Desktop;
    }

    // Default to Other for unrecognized User-Agents
    // This includes bots, crawlers, and obscure devices
    DeviceType::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    // iOS User-Agents
    const UA_IPHONE_SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1";
    const UA_IPAD_SAFARI: &str = "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1";
    const UA_IPOD: &str = "Mozilla/5.0 (iPod touch; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.0 Mobile/15E148 Safari/604.1";
    const UA_IPADOS_CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Mobile";

    // Android User-Agents
    const UA_ANDROID_CHROME: &str = "Mozilla/5.0 (Linux; Android 13; SM-S901B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Mobile Safari/537.36";
    const UA_ANDROID_TABLET: &str = "Mozilla/5.0 (Linux; Android 12; SM-T870) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36";
    const UA_ANDROID_FIREFOX: &str =
        "Mozilla/5.0 (Android 13; Mobile; rv:109.0) Gecko/20100101 Firefox/112.0";

    // Desktop User-Agents
    const UA_WINDOWS_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36";
    const UA_WINDOWS_FIREFOX: &str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/112.0";
    const UA_MACOS_SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.5 Safari/605.1.15";
    const UA_MACOS_CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36";
    const UA_LINUX_FIREFOX: &str =
        "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/112.0";
    const UA_LINUX_CHROME: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36";

    // Other/Bot User-Agents
    const UA_GOOGLEBOT: &str =
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
    const UA_CURL: &str = "curl/7.88.1";
    const UA_POSTMAN: &str = "PostmanRuntime/7.32.3";
    const UA_EMPTY: &str = "";

    #[test]
    fn test_detect_ios_iphone() {
        assert_eq!(detect_device(UA_IPHONE_SAFARI), DeviceType::IOS);
    }

    #[test]
    fn test_detect_ios_ipad() {
        assert_eq!(detect_device(UA_IPAD_SAFARI), DeviceType::IOS);
    }

    #[test]
    fn test_detect_ios_ipod() {
        assert_eq!(detect_device(UA_IPOD), DeviceType::IOS);
    }

    #[test]
    fn test_detect_ios_ipados_chrome() {
        // iPad Pro with iPadOS may report as Macintosh with Mobile indicator
        assert_eq!(detect_device(UA_IPADOS_CHROME), DeviceType::IOS);
    }

    #[test]
    fn test_detect_android_phone() {
        assert_eq!(detect_device(UA_ANDROID_CHROME), DeviceType::Android);
    }

    #[test]
    fn test_detect_android_tablet() {
        assert_eq!(detect_device(UA_ANDROID_TABLET), DeviceType::Android);
    }

    #[test]
    fn test_detect_android_firefox() {
        assert_eq!(detect_device(UA_ANDROID_FIREFOX), DeviceType::Android);
    }

    #[test]
    fn test_detect_desktop_windows_chrome() {
        assert_eq!(detect_device(UA_WINDOWS_CHROME), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_desktop_windows_firefox() {
        assert_eq!(detect_device(UA_WINDOWS_FIREFOX), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_desktop_macos_safari() {
        assert_eq!(detect_device(UA_MACOS_SAFARI), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_desktop_macos_chrome() {
        assert_eq!(detect_device(UA_MACOS_CHROME), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_desktop_linux_firefox() {
        assert_eq!(detect_device(UA_LINUX_FIREFOX), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_desktop_linux_chrome() {
        assert_eq!(detect_device(UA_LINUX_CHROME), DeviceType::Desktop);
    }

    #[test]
    fn test_detect_other_bots() {
        assert_eq!(detect_device(UA_GOOGLEBOT), DeviceType::Other);
        assert_eq!(detect_device(UA_CURL), DeviceType::Other);
        assert_eq!(detect_device(UA_POSTMAN), DeviceType::Other);
    }

    #[test]
    fn test_detect_other_empty() {
        assert_eq!(detect_device(UA_EMPTY), DeviceType::Other);
    }

    #[test]
    fn test_device_type_as_str() {
        assert_eq!(DeviceType::IOS.as_str(), "ios");
        assert_eq!(DeviceType::Android.as_str(), "android");
        assert_eq!(DeviceType::Desktop.as_str(), "desktop");
        assert_eq!(DeviceType::Other.as_str(), "other");
    }
}
