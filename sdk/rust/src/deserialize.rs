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
 *   This file is dual-licensed as Apache-2.0 or GPL-3.0.   *
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

use super::KEY_VALUE;

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
