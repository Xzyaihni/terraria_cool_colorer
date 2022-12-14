use std::fmt;


#[derive(Clone)]
pub enum Interpolation
{
    Random,
    Nearest,
    Linear,
    Cubic
}

#[derive(Clone)]
pub struct Color
{
    r: u8,
    g: u8,
    b: u8
}

impl Color
{
    pub fn new(r: u8, g: u8, b: u8) -> Self
    {
        Color{r, g, b}
    }

    pub fn interpolate(&self, other: &Color, amount: f32, interpolation: &Interpolation) -> Color
    {
        match interpolation
        {
            Interpolation::Random =>
            {
                self.interpolate_inner(other, |lhs, rhs|
                {
                    if rand::random::<f32>()<0.5
                    {
                        lhs
                    } else
                    {
                        rhs
                    }
                })
            },
            Interpolation::Nearest =>
            {
                self.interpolate_inner(other, |lhs, rhs|
                {
                    if amount<0.5
                    {
                        lhs
                    } else
                    {
                        rhs
                    }
                })
            },
            Interpolation::Linear =>
            {
                self.interpolate_inner(other, |lhs, rhs|
                {
                    let diff = rhs as i32 - lhs as i32;
                    let result = lhs as f32 + diff as f32*amount;

                    result.round() as u8
                })
            },
            Interpolation::Cubic =>
            {
                self.interpolate_inner(other, |lhs, rhs|
                {
                    todo!()
                })
            }
        }
    }

    fn interpolate_inner<F: FnMut(u8, u8) -> u8>(&self, other: &Color, mut interp: F) -> Color
    {
        Color{
            r: interp(self.r, other.r),
            g: interp(self.g, other.g),
            b: interp(self.b, other.b)
            }
    }
}

impl TryFrom<[&str; 3]> for Color
{
    type Error = String;

    fn try_from(item: [&str; 3]) -> Result<Self, Self::Error>
    {
        let parse = |color: &str| -> Result<u8, String>
        {
            color.trim().parse().map_err(|_| format!("error parsing {}", color))
        };

        Ok(Color{
            r: parse(item[0])?,
            g: parse(item[1])?,
            b: parse(item[2])?
            })
    }
}

impl fmt::Display for Color
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}


pub struct Colorer
{
    colors: Vec<Color>,
    shift: Option<f32>,
    interpolation: Interpolation,
    repeat: f32
}

impl Colorer
{
    pub fn new(colors: Vec<Color>, shift: bool, interpolation: Interpolation, repeat: f32) -> Self
    {
        if colors.is_empty()
        {
            panic!("colors cannot be empty");
        }

        let shift = if shift
        {
            Some(0.0)
        } else
        {
            None
        };

        let mut out = Colorer{colors, shift, interpolation, repeat};
        out.word();

        out
    }

    pub fn color_text(&mut self, text: &str) -> String
    {
        let chars_amount = text.chars().count();

        let mut new_message = String::new();

        let mut index = 0;
        let mut ignore = false;

        if let Some(color) = self.solid()
        {
            let color_string = format!("[c/{color}:");

            let mut iter = text.chars().peekable();

            ignore = iter.peek().map_or(true, |val| *val=='[');
            if !ignore
            {
                new_message.push_str(&color_string);
            }

            while let Some(c) = iter.next()
            {
                if c=='['
                {
                    if !ignore
                    {
                        new_message.push(']');
                    }
                    new_message.push(c);
                    ignore = true;
                } else
                {
                    new_message.push(c);

                    if c==']' && iter.peek().map_or(false, |val| *val!='[')
                    {
                        new_message.push_str(&color_string);
                        ignore = false;
                    }
                }
            }

            if !ignore
            {
                new_message.push(']');
            }
        } else
        {
            //signal that its a new message
            self.word();
            for c in text.chars()
            {
                if c=='['
                {
                    ignore = true;
                }

                if !ignore
                {
                    let position = index as f32/chars_amount as f32;

                    let colored = self.color(c, position);

                    new_message.push_str(colored.as_str());

                    index += 1;
                } else
                {
                    new_message.push(c);
                }

                if c==']'
                {
                    ignore = false;
                }
            }
        }

        new_message
    }

    fn solid(&self) -> Option<Color>
    {
        if self.colors.len()==1
        {
            Some(self.colors[0].clone())
        } else
        {
            None
        }
    }

    fn word(&mut self)
    {
        if self.shift.is_some()
        {
            self.shift = Some(rand::random());
        }
    }

    fn color(&self, c: char, position: f32) -> String
    {
        if c==' '
        {
            return c.to_string();
        }

        let color = if self.colors.len()==1
        {
            self.colors[0].clone()
        } else
        {
            let mut position = position*self.repeat;

            if let Some(amount) = self.shift
            {
                position += amount
            }

            if position>=self.repeat
            {
                position -= self.repeat;
            }

            let max_val = if self.shift.is_none()
            {
                self.colors.len()-1
            } else
            {
                self.colors.len()
            };

            let color_position = max_val as f32 * position;

            self.interpolate(
                color_position.floor() as usize % self.colors.len(),
                color_position.ceil() as usize % self.colors.len(),
                color_position.fract()
                )
        };

        format!("[c/{color}:{c}]")
    }

    fn interpolate(&self, left: usize, mut right: usize, amount: f32) -> Color
    {
        if right>=self.colors.len()
        {
            //could subtract self.colors.len() but it should never be more than len
            right = 0;
        }

        self.colors[left].interpolate(&self.colors[right], amount, &self.interpolation)
    }
}
