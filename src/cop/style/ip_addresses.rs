use crate::cop::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for hardcoded IP addresses in string literals.
///
/// ## Investigation findings (2026-03-15, updated 2026-03-18)
///
/// Root causes of 217 FPs and 66 FNs:
///
/// **FP causes (fixed):**
/// 1. String segments inside interpolated strings (`"#{x}10.0.0.1#{y}"`) lack
///    opening delimiters. RuboCop's `StringHelp` checks `node.loc?(:begin)` and
///    skips these; we now check `opening_loc().is_some()`.
/// 2. IPv4 validation accepted octets with leading zeros (e.g., `01.02.03.04`).
///    Ruby's `Resolv::IPv4::Regex` rejects leading zeros; we now match that behavior
///    with a strict octet validator.
/// 3. Used `unescaped()` instead of `content_loc().as_slice()`, so escape sequences
///    like `"\x31.2.3.4"` would expand to `1.2.3.4` and false-positive. RuboCop
///    checks the raw source between quotes (`node.source[1...-1]`).
/// 4. IPv6 compressed validator accepted `:::` (three colons), `:::A`, and `::A:`
///    as valid. `:::` is never valid IPv6 — added early rejection. `::A:` has a
///    trailing colon creating empty groups; fixed by requiring all groups in the
///    left/right halves of `::` split to be valid hex (no empty groups allowed).
///    These patterns appear in Ruby code as scope resolution operators (`:::`) and
///    IRB completion candidates.
///
/// **FN causes (fixed):**
/// 1. Missing IPv4-mapped IPv6 support (`::ffff:192.168.1.1`). Ruby's
///    `Resolv::IPv6::Regex` includes `Regex_6Hex4Dec` and `Regex_CompressedHex4Dec`
///    patterns for this format.
///
/// **Remaining FN (5):** No detailed location data available from corpus oracle.
/// Likely edge cases in IPv6 validation or config resolution differences.
pub struct IpAddresses;

/// Maximum length of an IPv6 address string (IPv4-mapped IPv6 is longest).
const IPV6_MAX_SIZE: usize = 45;

impl IpAddresses {
    /// Validate an IPv4 octet matching Ruby's `Resolv::IPv4::Regex256` pattern.
    /// Rejects leading zeros (e.g., "01", "001") to match Ruby's behavior.
    fn is_valid_ipv4_octet(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        // Reject leading zeros: "0" is ok, "00", "01", "001" etc. are not
        if s.len() > 1 && s.starts_with('0') {
            return false;
        }
        matches!(s.parse::<u16>(), Ok(n) if n <= 255)
    }

    fn is_ipv4(s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        parts.iter().all(|part| Self::is_valid_ipv4_octet(part))
    }

    /// Check if a hex group (colon-separated) is valid for IPv6.
    fn is_valid_hex_group(group: &str) -> bool {
        !group.is_empty() && group.len() <= 4 && group.bytes().all(|b| b.is_ascii_hexdigit())
    }

    fn is_ipv6(s: &str) -> bool {
        // Must not be too long
        if s.len() > IPV6_MAX_SIZE {
            return false;
        }

        // Must contain at least one colon
        if !s.contains(':') {
            return false;
        }

        // Three or more consecutive colons is never valid IPv6
        if s.contains(":::") {
            return false;
        }

        // Try IPv4-mapped IPv6 formats first (e.g., ::ffff:192.168.1.1)
        if s.contains('.') {
            return Self::is_ipv6_with_ipv4(s);
        }

        // Must only contain hex digits and colons
        if !s.bytes().all(|b| b.is_ascii_hexdigit() || b == b':') {
            return false;
        }

        // Check for :: (collapsed zeros)
        if s.contains("::") {
            return Self::is_ipv6_compressed(s);
        }

        // Full form: 8 groups of hex
        let groups: Vec<&str> = s.split(':').collect();
        if groups.len() != 8 {
            return false;
        }
        groups.iter().all(|g| Self::is_valid_hex_group(g))
    }

    /// Validate compressed IPv6 (contains ::)
    fn is_ipv6_compressed(s: &str) -> bool {
        // Can have at most one ::
        if s.matches("::").count() > 1 {
            return false;
        }
        let parts: Vec<&str> = s.split("::").collect();
        if parts.len() != 2 {
            return false;
        }
        // Validate left side: if non-empty, split by ':' and all groups must be valid hex
        // (no empty groups allowed — that would indicate a leading/trailing/extra colon)
        let left_groups: Vec<&str> = if parts[0].is_empty() {
            vec![]
        } else {
            let groups: Vec<&str> = parts[0].split(':').collect();
            for g in &groups {
                if !Self::is_valid_hex_group(g) {
                    return false;
                }
            }
            groups
        };
        // Validate right side similarly
        let right_groups: Vec<&str> = if parts[1].is_empty() {
            vec![]
        } else {
            let groups: Vec<&str> = parts[1].split(':').collect();
            for g in &groups {
                if !Self::is_valid_hex_group(g) {
                    return false;
                }
            }
            groups
        };
        if left_groups.len() + right_groups.len() > 7 {
            return false;
        }
        true
    }

    /// Validate IPv6 addresses with embedded IPv4 (e.g., ::ffff:192.168.1.1,
    /// 64:ff9b::192.0.2.33). Matches Ruby's Resolv::IPv6::Regex_6Hex4Dec and
    /// Regex_CompressedHex4Dec patterns.
    fn is_ipv6_with_ipv4(s: &str) -> bool {
        // Find the IPv4 part: everything after the last colon.
        // The IPv4 address is always at the end after a colon separator.
        let last_colon = match s.rfind(':') {
            Some(pos) => pos,
            None => return false,
        };
        // The prefix includes the trailing colon for easier parsing
        let ipv6_prefix = &s[..=last_colon];
        let ipv4_suffix = &s[last_colon + 1..];

        // The IPv4 suffix must be a valid IPv4 address
        if !Self::is_ipv4(ipv4_suffix) {
            return false;
        }

        // The prefix must only contain hex digits and colons
        if !ipv6_prefix
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b':')
        {
            return false;
        }

        if ipv6_prefix.contains("::") {
            // Compressed form with IPv4 (Regex_CompressedHex4Dec)
            if ipv6_prefix.matches("::").count() > 1 {
                return false;
            }
            let parts: Vec<&str> = ipv6_prefix.split("::").collect();
            if parts.len() != 2 {
                return false;
            }
            let left_groups: Vec<&str> = if parts[0].is_empty() {
                vec![]
            } else {
                let groups: Vec<&str> = parts[0].split(':').collect();
                for g in &groups {
                    if !Self::is_valid_hex_group(g) {
                        return false;
                    }
                }
                groups
            };
            let right_part = parts[1].trim_end_matches(':');
            let right_groups: Vec<&str> = if right_part.is_empty() {
                vec![]
            } else {
                let groups: Vec<&str> = right_part.split(':').collect();
                for g in &groups {
                    if !Self::is_valid_hex_group(g) {
                        return false;
                    }
                }
                groups
            };
            // IPv4 counts as 2 groups, so hex groups + 2 <= 8
            if left_groups.len() + right_groups.len() > 5 {
                return false;
            }
            true
        } else {
            // Full form: exactly 6 hex groups (Regex_6Hex4Dec)
            // The prefix has a trailing ":" so strip it before splitting.
            let prefix_trimmed = ipv6_prefix.trim_end_matches(':');
            let groups: Vec<&str> = prefix_trimmed.split(':').collect();
            if groups.len() != 6 {
                return false;
            }
            groups.iter().all(|g| Self::is_valid_hex_group(g))
        }
    }

    /// Pre-filter: the first character must be a hex digit or colon.
    /// Matches RuboCop's `starts_with_hex_or_colon?` optimization.
    fn starts_with_hex_or_colon(s: &str) -> bool {
        match s.as_bytes().first() {
            Some(b) => b.is_ascii_hexdigit() || *b == b':',
            None => false,
        }
    }
}

impl Cop for IpAddresses {
    fn name(&self) -> &'static str {
        "Style/IpAddresses"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        // Skip string segments inside interpolated strings (no opening delimiter).
        // Matches RuboCop's StringHelp `node.loc?(:begin)` check.
        if string_node.opening_loc().is_none() {
            return;
        }

        // Use raw source content between quotes (not unescaped) to match
        // RuboCop's `node.source[1...-1]` behavior. This avoids FPs from
        // escape sequences like "\x31.2.3.4" expanding to "1.2.3.4".
        let content_loc = string_node.content_loc();
        let content_bytes = content_loc.as_slice();
        let content_str = match std::str::from_utf8(content_bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        if content_str.is_empty() {
            return;
        }

        // Quick pre-filter: string too long or doesn't start with hex/colon
        if content_str.len() > IPV6_MAX_SIZE || !Self::starts_with_hex_or_colon(content_str) {
            return;
        }

        let allowed = config
            .get_string_array("AllowedAddresses")
            .or_else(|| Some(vec!["::".to_string()]));

        // Check if it's in allowed addresses (case-insensitive)
        if let Some(ref allowed_list) = allowed {
            let content_lower = content_str.to_lowercase();
            for addr in allowed_list {
                if addr.to_lowercase() == content_lower {
                    return;
                }
            }
        }

        let is_ip = Self::is_ipv4(content_str) || Self::is_ipv6(content_str);

        if is_ip {
            let loc = string_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not hardcode IP addresses.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IpAddresses, "cops/style/ip_addresses");

    #[test]
    fn test_ipv4_validation() {
        assert!(IpAddresses::is_ipv4("127.0.0.1"));
        assert!(IpAddresses::is_ipv4("0.0.0.0"));
        assert!(IpAddresses::is_ipv4("255.255.255.255"));
        assert!(IpAddresses::is_ipv4("192.168.1.1"));

        // Leading zeros rejected (matching Ruby's Resolv::IPv4::Regex)
        assert!(!IpAddresses::is_ipv4("01.02.03.04"));
        assert!(!IpAddresses::is_ipv4("001.002.003.004"));
        assert!(!IpAddresses::is_ipv4("1.2.3.04"));
        assert!(!IpAddresses::is_ipv4("192.168.001.001"));
        assert!(!IpAddresses::is_ipv4("10.0.0.01"));

        // Invalid
        assert!(!IpAddresses::is_ipv4("578.194.591.059"));
        assert!(!IpAddresses::is_ipv4("1.2.3"));
        assert!(!IpAddresses::is_ipv4("1.2.3.4.5"));
        assert!(!IpAddresses::is_ipv4(""));
    }

    #[test]
    fn test_ipv6_validation() {
        // Full form
        assert!(IpAddresses::is_ipv6(
            "2001:0db8:85a3:0000:0000:8a2e:0370:7334"
        ));
        assert!(IpAddresses::is_ipv6("0:0:0:0:0:0:0:0"));
        assert!(IpAddresses::is_ipv6("0:0:0:0:0:0:0:1"));

        // Compressed
        assert!(IpAddresses::is_ipv6("2001:db8::1"));
        assert!(IpAddresses::is_ipv6("::1"));
        assert!(IpAddresses::is_ipv6("1::"));
        assert!(IpAddresses::is_ipv6("::"));
        assert!(IpAddresses::is_ipv6("2001:db8:85a3::8a2e:370:7334"));
        assert!(IpAddresses::is_ipv6("::ffff:0:0"));

        // IPv4-mapped
        assert!(IpAddresses::is_ipv6("::ffff:192.168.1.1"));
        assert!(IpAddresses::is_ipv6("64:ff9b::192.0.2.33"));

        // Invalid
        assert!(!IpAddresses::is_ipv6("2001:db8::1xyz"));
        assert!(!IpAddresses::is_ipv6(""));
        assert!(!IpAddresses::is_ipv6("not-an-ip"));

        // Triple colons and related patterns are NOT valid IPv6
        assert!(!IpAddresses::is_ipv6(":::"));
        assert!(!IpAddresses::is_ipv6(":::A"));
        assert!(!IpAddresses::is_ipv6("::A:"));
        assert!(!IpAddresses::is_ipv6(":::A:"));
    }

    #[test]
    fn test_ipv4_octet_no_leading_zeros() {
        assert!(IpAddresses::is_valid_ipv4_octet("0"));
        assert!(IpAddresses::is_valid_ipv4_octet("1"));
        assert!(IpAddresses::is_valid_ipv4_octet("255"));
        assert!(!IpAddresses::is_valid_ipv4_octet("00"));
        assert!(!IpAddresses::is_valid_ipv4_octet("01"));
        assert!(!IpAddresses::is_valid_ipv4_octet("001"));
        assert!(!IpAddresses::is_valid_ipv4_octet("256"));
        assert!(!IpAddresses::is_valid_ipv4_octet(""));
    }
}
