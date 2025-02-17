#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use camino::Utf8PathBuf;
    use chrono::{DateTime, Utc};
    use serde::Deserialize;
    use sqlx::{FromRow, SqlitePool};
    use starknet::core::types::FieldElement;

    use crate::state::sql::{Executable, Sql};
    use crate::state::State;
    use crate::tests::common::run_graphql_query;

    #[derive(Deserialize)]
    struct Moves {
        __typename: String,
        remaining: String,
    }

    #[derive(Deserialize)]
    struct Position {
        __typename: String,
        x: String,
        y: String,
    }

    #[derive(Deserialize)]
    struct ComponentMoves {
        name: String,
        storage: Moves,
    }

    #[derive(Deserialize)]
    struct ComponentPosition {
        name: String,
        storage: Position,
    }

    #[derive(FromRow, Deserialize)]
    pub struct Component {
        pub id: String,
        pub name: String,
        pub class_hash: String,
        pub transaction_hash: String,
        pub created_at: DateTime<Utc>,
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_storage_components(pool: SqlitePool) {
        component_fixtures(&pool).await;

        let query = r#"
                {
                    moves {
                        __typename
                        remaining
                    }
                    position {
                        __typename
                        x
                        y
                    }
                }
            "#;
        let value = run_graphql_query(&pool, query).await;

        let moves = value.get("moves").ok_or("no moves found").unwrap();
        let moves: Moves = serde_json::from_value(moves.clone()).unwrap();
        let position = value.get("position").ok_or("no position found").unwrap();
        let position: Position = serde_json::from_value(position.clone()).unwrap();

        assert_eq!(moves.remaining, "0xa");
        assert_eq!(position.x, "0x2a");
        assert_eq!(position.y, "0x45");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_storage_union(pool: SqlitePool) {
        component_fixtures(&pool).await;

        let query = r#"
                { 
                    component_moves: component(id: "moves") {
                        name
                        storage {
                            __typename
                            ... on Moves {
                                remaining
                            }
                        }
                    }
                    component_position: component(id: "position") {
                        name
                        storage {
                            __typename
                            ... on Position {
                                x
                                y
                            }
                        }
                    }
                }
            "#;
        let value = run_graphql_query(&pool, query).await;
        let component_moves = value.get("component_moves").ok_or("no component found").unwrap();
        let component_moves: ComponentMoves =
            serde_json::from_value(component_moves.clone()).unwrap();
        let component_position =
            value.get("component_position").ok_or("no component found").unwrap();
        let component_position: ComponentPosition =
            serde_json::from_value(component_position.clone()).unwrap();

        assert_eq!(component_moves.name, component_moves.storage.__typename);
        assert_eq!(component_position.name, component_position.storage.__typename);
    }

    async fn component_fixtures(pool: &SqlitePool) {
        let manifest = dojo_world::manifest::Manifest::load_from_path(
            Utf8PathBuf::from_path_buf("../../examples/ecs/target/dev/manifest.json".into())
                .unwrap(),
        )
        .unwrap();

        let state = Sql::new(pool.clone(), FieldElement::ZERO).await.unwrap();
        state.load_from_manifest(manifest).await.unwrap();

        // Set moves entity
        let key = FieldElement::ONE;
        let partition = FieldElement::from_hex_be("0xdead").unwrap();
        let values =
            HashMap::from([(String::from("remaining"), FieldElement::from_hex_be("0xa").unwrap())]);
        state.set_entity("moves".to_string(), partition, key, values).await.unwrap();

        // Set position entity
        let key = FieldElement::TWO;
        let partition = FieldElement::from_hex_be("0xbeef").unwrap();
        let values = HashMap::from([
            (String::from("x"), FieldElement::from_hex_be("0x2a").unwrap()),
            (String::from("y"), FieldElement::from_hex_be("0x45").unwrap()),
        ]);
        state.set_entity("position".to_string(), partition, key, values).await.unwrap();
        state.execute().await.unwrap();
    }
}
