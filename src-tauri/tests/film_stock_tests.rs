mod common;

use kammerz_lib::entities::film_stock::{self, FilmFormat, FilmStockType};
use kammerz_lib::services::film_stock_service::FilmStockService;
use sea_orm::Set;

#[tokio::test]
async fn create_and_read_film_stock() {
    let db = common::setup_db().await;
    let stock = common::create_test_film_stock(&db).await;

    assert_eq!(stock.brand, "TestFilm");
    assert_eq!(stock.name, "Test 400");
    assert_eq!(stock.format, FilmFormat::F135);
    assert_eq!(stock.stock_type, FilmStockType::BwNegative);
    assert_eq!(stock.iso, Some(400));
    assert_eq!(stock.exposure_count, Some(36));

    let fetched = FilmStockService::get_by_id(&db, stock.id).await.unwrap();
    assert!(fetched.is_some());
}

#[tokio::test]
async fn update_film_stock() {
    let db = common::setup_db().await;
    let stock = common::create_test_film_stock(&db).await;

    let mut model: film_stock::ActiveModel = stock.into();
    model.name = Set("Updated 800".into());
    model.iso = Set(Some(800));
    model.stock_type = Set(FilmStockType::ColorNegative);

    let updated = FilmStockService::update(&db, model).await.unwrap();
    assert_eq!(updated.name, "Updated 800");
    assert_eq!(updated.iso, Some(800));
    assert_eq!(updated.stock_type, FilmStockType::ColorNegative);
}

#[tokio::test]
async fn delete_film_stock() {
    let db = common::setup_db().await;
    let stock = common::create_test_film_stock(&db).await;

    FilmStockService::delete(&db, stock.id).await.unwrap();

    let fetched = FilmStockService::get_by_id(&db, stock.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn film_120_has_null_exposure_count() {
    let db = common::setup_db().await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let stock = FilmStockService::create(
        &db,
        film_stock::ActiveModel {
            brand: Set("TestBrand120".into()),
            name: Set("MedFormat 400".into()),
            format: Set(FilmFormat::F120),
            stock_type: Set(FilmStockType::ColorNegative),
            iso: Set(Some(400)),
            exposure_count: Set(None), // 120 film: frame count depends on back size
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(stock.format, FilmFormat::F120);
    assert_eq!(stock.exposure_count, None, "120 film should have NULL exposure_count");
}

#[tokio::test]
async fn sheet_film_has_exposure_count_1() {
    let db = common::setup_db().await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let stock = FilmStockService::create(
        &db,
        film_stock::ActiveModel {
            brand: Set("Kodak".into()),
            name: Set("Tri-X 320".into()),
            format: Set(FilmFormat::Sheet4x5),
            stock_type: Set(FilmStockType::BwNegative),
            iso: Set(Some(320)),
            exposure_count: Set(Some(1)), // One sheet per holder side
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(stock.format, FilmFormat::Sheet4x5);
    assert_eq!(stock.exposure_count, Some(1));
}

#[tokio::test]
async fn distinct_brands_includes_seeded() {
    let db = common::setup_db().await;

    let brands = FilmStockService::distinct_brands(&db).await.unwrap();
    // Migrations seed Kodak, Ilford, Fujifilm, etc.
    assert!(!brands.is_empty(), "Should include seeded film stock brands");
    assert!(
        brands.iter().any(|b| b == "Kodak"),
        "Seeded brands should include Kodak"
    );
}

#[tokio::test]
async fn list_all_includes_seeded_stocks() {
    let db = common::setup_db().await;

    let stocks = FilmStockService::list_all(&db).await.unwrap();
    // Migrations seed many film stocks
    assert!(stocks.len() > 5, "Should have seeded film stocks from migrations");
}

#[tokio::test]
async fn all_film_format_variants_round_trip() {
    let db = common::setup_db().await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let formats = [
        FilmFormat::F135,
        FilmFormat::F120,
        FilmFormat::Sheet4x5,
        FilmFormat::Sheet5x7,
        FilmFormat::Sheet8x10,
        FilmFormat::Instant,
    ];

    for (i, format) in formats.iter().enumerate() {
        let stock = FilmStockService::create(
            &db,
            film_stock::ActiveModel {
                brand: Set("EnumTest".into()),
                name: Set(format!("Format {i}")),
                format: Set(format.clone()),
                stock_type: Set(FilmStockType::BwNegative),
                created_at: Set(ts.clone()),
                updated_at: Set(ts.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let fetched = FilmStockService::get_by_id(&db, stock.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&fetched.format, format, "Format enum should round-trip through SQLite");
    }
}
