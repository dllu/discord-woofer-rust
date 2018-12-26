extern crate reqwest;
extern crate url;

#[derive(Deserialize)]
pub struct Location {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize)]
pub struct GeocodeGeometry {
    location: Location,
}

#[derive(Deserialize)]
pub struct GeocodeLocation {
    geometry: GeocodeGeometry,
}

#[derive(Deserialize)]
pub struct Geocode {
    results: Vec<GeocodeLocation>,
}

pub fn geocode(address: String, apikey: &String) -> Result<Location, reqwest::Error> {
    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("address", &address)
        .append_pair("key", &apikey)
        .finish();
    let geocode_url = format!(
        "https://maps.googleapis.com/maps/api/geocode/json?{}",
        encoded
    );
    let geocode_response: Geocode = reqwest::get(&geocode_url)?.json()?;
    let location = &geocode_response.results.first().unwrap().geometry.location;
    Ok(Location {
        lat: location.lat,
        lng: location.lng,
    })
}

#[derive(Deserialize)]
pub struct WeatherCurrently {
    summary: String,
    icon: String,
    temperature: f64,
    humidity: f64,
    precipProbability: f64,
}

#[derive(Deserialize)]
pub struct Weather {
    currently: WeatherCurrently,
}

pub fn weather(location: &Location, apikey: &String) -> Result<WeatherCurrently, reqwest::Error> {
    let forecast_url = format!(
        "https://api.darksky.net/forecast/{}/{},{}?units=si",
        apikey, location.lat, location.lng
    );
    println!("{}", forecast_url);
    let weather: Weather = reqwest::get(&forecast_url)?.json()?;
    Ok(weather.currently)
}

pub fn weather_string(
    address: String,
    location: &Location,
    weather_currently: WeatherCurrently,
) -> String {
    let emo = emoji(weather_currently.icon);
    format!("weather in {} ({}, {}): {}. Temperature {:.2} K. Humidity {:.1}%. Precipitation probability {:.1}%. {}",
       address,
       location.lat,
       location.lng,
       weather_currently.summary,
       weather_currently.temperature + 273.15_f64,
       weather_currently.humidity * 100_f64,
       weather_currently.precipProbability * 100_f64,
       emo)
}

fn emoji(icon: String) -> String {
    match icon.as_str() {
        "clear-day" => "☀️".to_string(),
        "clear-night" => "🌃".to_string(),
        "rain" => "🌧️".to_string(),
        "snow" => "🌨️".to_string(),
        "sleet" => "🌨️".to_string(),
        "wind" => "🌬️".to_string(),
        "fog" => "🌫️".to_string(),
        "cloudy" => "☁️".to_string(),
        "partly-cloudy-day" => "⛅".to_string(),
        "partly-cloudy-night" => "☁️".to_string(),
        "thunderstorm" => "🌩️".to_string(),
        _ => "".to_string(),
    }
}
