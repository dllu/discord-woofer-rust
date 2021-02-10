extern crate reqwest;
extern crate url;

#[derive(Deserialize, Debug)]
pub struct Location {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize, Debug)]
pub struct GeocodeGeometry {
    location: Location,
}

#[derive(Deserialize, Debug)]
pub struct GeocodeLocation {
    geometry: GeocodeGeometry,
}

#[derive(Deserialize, Debug)]
pub struct Geocode {
    results: Vec<GeocodeLocation>,
}

pub async fn geocode(address: String, apikey: &str) -> Result<Location, reqwest::Error> {
    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("address", &address)
        .append_pair("key", &apikey)
        .finish();
    let geocode_url = format!(
        "https://maps.googleapis.com/maps/api/geocode/json?{}",
        encoded
    );
    let geocode_response: Geocode = reqwest::get(&geocode_url).await?.json().await?;
    let location = &geocode_response.results.first().unwrap().geometry.location;
    Ok(Location {
        lat: location.lat,
        lng: location.lng,
    })
}

#[derive(Deserialize, Debug)]
pub struct WeatherMain {
    temp: f64,
    humidity: f64,
}

#[derive(Deserialize, Debug)]
pub struct WeatherWeather {
    description: String,
    icon: String,
}

#[derive(Deserialize, Debug)]
pub struct Weather {
    main: WeatherMain,
    weather: Vec<WeatherWeather>,
}

pub async fn weather(location: &Location, apikey: &str) -> Result<Weather, reqwest::Error> {
    let forecast_url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&lang=en",
        location.lat, location.lng, apikey
    );
    println!("{}", forecast_url);
    let weather: Weather = reqwest::get(&forecast_url).await?.json().await?;
    Ok(weather)
}

pub fn weather_string(address: String, location: &Location, weather: Weather) -> String {
    let emo = emoji(&weather.weather[0].icon);
    format!(
        "weather in {} ({}, {}): {}. Temperature {:.2} K. Humidity {:.1}%. {}",
        address,
        location.lat,
        location.lng,
        weather.weather[0].description,
        weather.main.temp,
        weather.main.humidity,
        emo
    )
}

fn emoji(icon: &str) -> String {
    match icon {
        "01d" => "☀️".to_string(),
        "01n" => "🌃".to_string(),
        "02d" => "⛅".to_string(),
        "02n" => "☁️".to_string(),
        "03d" => "☁️".to_string(),
        "03n" => "☁️".to_string(),
        "04d" => "⛅".to_string(),
        "04n" => "☁️".to_string(),
        "09d" => "🌧️".to_string(),
        "09n" => "🌧️".to_string(),
        "10d" => "🌧️".to_string(),
        "10n" => "🌧️".to_string(),
        "11d" => "🌩️".to_string(),
        "11n" => "🌩️".to_string(),
        "13d" => "🌨️".to_string(),
        "13n" => "🌨️".to_string(),
        "50d" => "🌫️".to_string(),
        "50n" => "🌫️".to_string(),
        _ => "".to_string(),
    }
}
