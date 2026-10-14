//! What QML is given and gives back: a serde value as the QVariant QML
//! reads as a JavaScript object or array, and the other way round.

use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QMetaTypeType, QString, QVariant};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Number, Value};

pub fn to_qml(value: &impl Serialize) -> QVariant {
    match serde_json::to_value(value) {
        Ok(value) => from_value(&value),
        Err(problem) => {
            rimstack_core::problem!("not given to the window: {problem}");
            QVariant::default()
        }
    }
}

/// `None` for what does not read as `T`; the reason goes to the record.
pub fn from_qml<T: DeserializeOwned>(given: &QVariant) -> Option<T> {
    serde_json::from_value(to_value(given))
        .map_err(|problem| rimstack_core::problem!("not read from the window: {problem}"))
        .ok()
}

fn from_value(value: &Value) -> QVariant {
    match value {
        Value::Null => QVariant::default(),
        Value::Bool(b) => QVariant::from(b),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => QVariant::from(&i),
            (None, Some(f)) => QVariant::from(&f),
            (None, None) => QVariant::default(),
        },
        Value::String(text) => QVariant::from(&QString::from(text.as_str())),
        Value::Array(items) => {
            let mut list = QList::<QVariant>::default();
            for item in items {
                list.append(from_value(item));
            }
            QVariant::from(&list)
        }
        Value::Object(fields) => {
            let mut map = QMap::<QMapPair_QString_QVariant>::default();
            for (name, field) in fields {
                map.insert(QString::from(name.as_str()), from_value(field));
            }
            QVariant::from(&map)
        }
    }
}

fn to_value(given: &QVariant) -> Value {
    let number = |n: Option<Number>| n.map_or(Value::Null, Value::Number);
    match given.type_id() {
        QMetaTypeType::Bool => Value::Bool(given.value_or_default::<bool>()),
        QMetaTypeType::Int => Value::from(given.value_or_default::<i32>()),
        QMetaTypeType::UInt => Value::from(given.value_or_default::<u32>()),
        QMetaTypeType::Short | QMetaTypeType::Long | QMetaTypeType::LongLong => {
            Value::from(given.value_or_default::<i64>())
        }
        QMetaTypeType::UShort | QMetaTypeType::ULong | QMetaTypeType::ULongLong => {
            Value::from(given.value_or_default::<u64>())
        }
        QMetaTypeType::Double => number(Number::from_f64(given.value_or_default::<f64>())),
        QMetaTypeType::Float => number(Number::from_f64(given.value_or_default::<f32>().into())),
        QMetaTypeType::QString => Value::String(given.value_or_default::<QString>().to_string()),
        QMetaTypeType::QStringList => {
            let texts = given.value_or_default::<cxx_qt_lib::QStringList>();
            let list: QList<QString> = QList::from(&texts);
            Value::Array(list.iter().map(|t| Value::String(t.to_string())).collect())
        }
        QMetaTypeType::QVariantList => {
            let list = given.value_or_default::<QList<QVariant>>();
            Value::Array(list.iter().map(to_value).collect())
        }
        QMetaTypeType::QVariantMap => {
            let map = given.value_or_default::<QMap<QMapPair_QString_QVariant>>();
            let fields: Map<String, Value> = map
                .iter()
                .map(|(name, field)| (name.to_string(), to_value(field)))
                .collect();
            Value::Object(fields)
        }
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Fix {
        action: String,
    }

    #[derive(Serialize, serde::Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Problem {
        mod_id: i64,
        error: bool,
        megabytes: Option<f64>,
        fix: Fix,
        tags: Vec<String>,
    }

    #[test]
    fn a_record_goes_to_the_window_and_comes_back_the_same() {
        let sent = vec![Problem {
            mod_id: 3,
            error: true,
            megabytes: Some(1.5),
            fix: Fix {
                action: "trust".into(),
            },
            tags: vec!["roads".into(), "é".into()],
        }];
        let given = to_qml(&sent);
        assert_eq!(given.type_id(), QMetaTypeType::QVariantList);
        assert_eq!(from_qml::<Vec<Problem>>(&given), Some(sent));
    }

    #[test]
    fn nothing_given_and_the_wrong_shape_read_as_none() {
        assert_eq!(from_qml::<Vec<i64>>(&QVariant::default()), None);
        assert_eq!(from_qml::<Vec<i64>>(&to_qml(&"text")), None);
        assert_eq!(from_qml::<Option<String>>(&QVariant::default()), Some(None));
    }

    #[test]
    fn numbers_keep_their_kind() {
        assert_eq!(from_qml::<i64>(&to_qml(&-7_i64)), Some(-7));
        assert_eq!(to_qml(&0.25_f64).type_id(), QMetaTypeType::Double);
        assert_eq!(from_qml::<f64>(&to_qml(&0.25_f64)), Some(0.25));
        assert_eq!(
            from_qml::<u64>(&to_qml(&3_000_000_000_u64)),
            Some(3_000_000_000)
        );
        assert_eq!(from_qml::<i32>(&QVariant::from(&5_i32)), Some(5));
    }
}
