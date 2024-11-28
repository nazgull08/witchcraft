use chrono::{DateTime, Utc};
use log::info;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Представление одной свечи (OHLCV).
#[derive(Debug, Clone)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>, // Время начала интервала свечи
}

/// Основной стор для хранения и управления свечами.
#[derive(Debug)]
pub struct CandleStore {
    // candles: symbol -> interval -> Vec<Candle>
    candles: RwLock<HashMap<String, HashMap<u64, Vec<Candle>>>>,
}

impl CandleStore {
    /// Создает новый пустой `CandleStore`.
    pub fn new() -> Self {
        Self {
            candles: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_price(&self, symbol: &str, interval: u64, price: f64, volume: f64, event_time: i64) {
        let mut candles = self.candles.write().unwrap();

        // Достаем свечи для указанного символа и интервала
        let symbol_candles = candles.entry(symbol.to_string()).or_insert_with(HashMap::new);
        let candle_list = symbol_candles.entry(interval).or_insert_with(Vec::new);

        // Рассчитываем начало периода на основе времени события
        let period_start = event_time - (event_time % interval as i64);

        // Проверяем последнюю свечу
        if let Some(last_candle) = candle_list.last() {
            let last_timestamp = last_candle.timestamp.timestamp();
            let last_close = last_candle.close;

            if last_timestamp == period_start {
                // Обновляем текущую свечу
                if let Some(last_candle) = candle_list.last_mut() {
                    last_candle.high = last_candle.high.max(price);
                    last_candle.low = last_candle.low.min(price);
                    last_candle.close = price;
                    last_candle.volume += volume;
                }
                return;
            } else if last_timestamp < period_start {
                // Добавляем пропущенные свечи
                let mut missing_start = last_timestamp + interval as i64;
                while missing_start < period_start {
                    let empty_candle = Candle {
                        open: last_close,
                        high: last_close,
                        low: last_close,
                        close: last_close,
                        volume: 0.0,
                        timestamp: DateTime::<Utc>::from_utc(
                            chrono::NaiveDateTime::from_timestamp(missing_start, 0),
                            Utc,
                        ),
                    };
                    candle_list.push(empty_candle);
                    missing_start += interval as i64;
                }
            }
        }

        // Создаем новую свечу
        let new_candle = Candle {
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            timestamp: DateTime::<Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(period_start, 0),
                Utc,
            ),
        };
        candle_list.push(new_candle);

        // Ограничиваем количество хранимых свечей
        const MAX_CANDLES: usize = 1000;
        if candle_list.len() > MAX_CANDLES {
            candle_list.drain(0..(candle_list.len() - MAX_CANDLES));
        }
    }

    /// Получает последние `count` свечей для заданного символа и интервала.
    pub fn get_candles(
        &self,
        symbol: &str,
        interval: u64,
        count: usize,
    ) -> Vec<Candle> {
        let candles = self.candles.read().unwrap();
        if let Some(symbol_candles) = candles.get(symbol) {
            if let Some(interval_candles) = symbol_candles.get(&interval) {
                return interval_candles.iter().rev().take(count).cloned().collect();
            }
        }
        vec![]
    }

    pub fn get_candles_in_time_range_mils(
        &self,
        symbol: &str,
        interval: u64,
        from: u64,
        to: u64,
    ) -> Vec<Candle> {
        let candles = self.candles.read().unwrap();
        if let Some(interval_candles) = candles
            .get(symbol)
            .and_then(|interval_map| interval_map.get(&interval))
        {
            let filtered: Vec<Candle> = interval_candles
                .iter()
                .filter(|c| {
                    let timestamp = c.timestamp.timestamp() as u64;
                    let result = timestamp >= from && timestamp <= to;
                    info!(
                        "Filtering candle: {:?}, timestamp: {}, result: {}",
                        c, timestamp, result
                    );
                    result
                })
                .cloned()
                .collect();

            info!("Filtered candles: {:?}", filtered);
            filtered
        } else {
            info!(
                "No candles found for symbol: {}, interval: {}, from: {}, to: {}",
                symbol, interval, from, to
            );
            vec![]
        }
    }

    pub fn get_candles_in_time_range_secs(
        &self,
        symbol: &str,
        interval: u64,
        from: u64,
        to: u64,
    ) -> Vec<Candle> {
        let candles = self.candles.read().unwrap();
        if let Some(interval_candles) = candles
            .get(symbol)
            .and_then(|interval_map| interval_map.get(&interval))
        {
            let filtered: Vec<Candle> = interval_candles
                .iter()
                .filter(|c| {
                    let timestamp = c.timestamp.timestamp() as u64;
                    let result = timestamp >= from && timestamp <= to;
                    info!(
                        "Filtering candle: {:?}, timestamp: {}, result: {}",
                        c, timestamp, result
                    );
                    result
                })
                .cloned()
                .collect();

            info!("Filtered candles: {:?}", filtered);
            filtered
        } else {
            info!(
                "No candles found for symbol: {}, interval: {}, from: {}, to: {}",
                symbol, interval, from, to
            );
            vec![]
        }
    }

    pub fn get_min_max_timestamps(&self) -> Option<(i64, i64)> {
        let candles = self.candles.read().unwrap();
        if candles.is_empty() {
            return None;
        }

        // Собираем все timestamp из всех свечей
        let timestamps: Vec<i64> = candles
            .values() // Доступ ко всем HashMap<u64, Vec<Candle>>
            .flat_map(|interval_map| interval_map.values()) // Доступ ко всем Vec<Candle>
            .flat_map(|candle_list| candle_list.iter().map(|candle| candle.timestamp.timestamp())) // Берем timestamp из свечей
            .collect();

        // Ищем минимальный и максимальный timestamp
        let min = timestamps.iter().min().cloned()?;
        let max = timestamps.iter().max().cloned()?;

        Some((min, max))
    }

}
