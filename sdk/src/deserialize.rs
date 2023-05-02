/* -------------------------------------------------------- *\
 *                                                          *
 *      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
 *      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
 *      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
 *      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
 *      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
 *      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
 *                                         by Nutshimit     *
 * -------------------------------------------------------- *
 *                                                          *
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

use super::KEY_VALUE;

/// This function is a custom deserializer for resource state fields. It checks if the field is an
/// object containing a `__value` key. If the `__value` key is present, its associated value is used
/// for deserialization, otherwise the entire object is used.
///
/// ***
///
/// ### Type Parameters
///
/// * `D`: The deserializer type, implementing the `serde::Deserializer` trait.
/// * `T`: The target type to deserialize the resource state field into, implementing the `serde::Deserialize` trait.
///
/// ***
///
/// ### Arguments
///
/// * `deserializer`: The deserializer instance to use for deserializing the resource state field.
///
/// ***
///
/// ### Returns
///
/// A `Result<T, D::Error>` with the deserialized value of the field or an error if deserialization fails.
///
pub fn deserialize_state_field<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
	D: serde::Deserializer<'de>,
	T: for<'a> serde::Deserialize<'a>,
{
	struct ValueFieldVisitor;

	impl<'de> serde::de::Visitor<'de> for ValueFieldVisitor {
		type Value = serde_json::Value;

		fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
			formatter.write_str("a resource state")
		}

		fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
		where
			A: serde::de::MapAccess<'de>,
		{
			let mut value_opt = None;

			while let Some(key) = map.next_key::<String>()? {
				if key == KEY_VALUE {
					value_opt = Some(map.next_value()?);
				} else {
					let _ = map.next_value::<serde_json::Value>()?;
				}
			}

			value_opt.ok_or_else(|| serde::de::Error::custom("Missing 'value' field"))
		}
	}

	deserializer
		.deserialize_map(ValueFieldVisitor)
		.and_then(|value| serde_json::from_value::<T>(value).map_err(serde::de::Error::custom))
}
