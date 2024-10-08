# discord-woofer-rust

a rust discord bot

supported commands:

* `woof` echos a dog-like onomatopoeia
* `puppy weather [unit] [place name]` get current weather (powered by [OpenWeather API](https://openweathermap.org/api)). 
    * Note, providing ``unit`` is optional, however the following options are supported:
        * For Kelvin use ``kelvin`` or leave blank
        * For Celsius use ``celsius``
        * For Fahrenheit use ``fahrenheit``

![image](https://github.com/dllu/discord-woofer-rust/assets/14482624/32deb318-08b2-4b0a-b6ad-e7714dc569ca)

* `puppy why` makes a random excuse

![image](https://github.com/dllu/discord-woofer-rust/assets/14482624/0a24311c-9dda-46fd-bd7f-9d91f75ffbb2)

* `puppy stonk [stock ticker]` checks the stock price, e.g. `tsla`

![image](https://github.com/dllu/discord-woofer-rust/assets/14482624/881b80f2-6775-478b-b866-f78e7451acdc)

* `puppy chess [algebraic chess notation]` plays a game of chess with other people in the channel, e.g. `e4`

![image](https://github.com/dllu/discord-woofer-rust/assets/14482624/59e5c0cd-a531-4ce7-84d5-8077dd9ae5ef)

* `puppy gpt [question]` asks a question to `mixtral-8x7b-32768` via the [Groq API](https://console.groq.com/)

![image](https://github.com/dllu/discord-woofer-rust/assets/14482624/2f0228dc-5c3f-4026-a353-1e61e47e5886)
