# rmatrix

[![Rust](https://github.com/mov-rax-rbx/rmatrix/actions/workflows/rust.yml/badge.svg)](https://github.com/mov-rax-rbx/rmatrix/actions/workflows/rust.yml)

Terminal `matrix` rain.


![](gif/preview1.gif)
![](gif/preview2.gif)
![](gif/preview3.gif)

# Hot reloading config file `config.rm`

```
# all general properties (# - comment)

# speed range of individual rain
speed :: 1..3

# length range of individual rain
length :: 5..20

# rain color, can be either a tuple or a range of tuples
# color :: (0, 0, 0)..(0, 255, 0)
# color :: (255, 0, 0)..(128, 0, 128)
color :: (0, 200, 0)

# the color of the last character
# head_color :: (255, 0, 255)
head_color :: (255, 255, 255)

# the coefficient that determines the uniformity of interpolation of the color
# interpolate_color_koef :: 1.5
interpolate_color_koef :: nil

# minimum brightness for rain
# min_brightnes :: 0.1
min_brightnes :: nil

# rain factor
density :: 0.7

is_bold :: true
is_default_rain :: true

# rain update time delay in milliseconds
delay :: 16

# set utf8 symbols
utf8 :: true
```

# How execute?

``` console
> git clone https://github.com/mov-rax-rbx/rmatrix.git
> cd rmatrix
> cargo r --release
```

# Dependecies

* [crossterm](https://github.com/crossterm-rs/crossterm) - cross-platform terminal manipulation library.
* [notify](https://github.com/notify-rs/notify) - cross-platform filesystem notification library.
* [rand](https://github.com/rust-random/rand) - library for random number generation.
