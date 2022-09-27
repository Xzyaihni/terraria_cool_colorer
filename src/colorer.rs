use std::fmt;


#[derive(Clone)]
pub enum Interpolation
{
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
            Interpolation::Nearest =>
            {
                let interpolate = |lhs, rhs|
                {
                    if amount<0.5
                    {
                        lhs
                    } else
                    {
                        rhs
                    }
                };

                Color{
                    r: interpolate(self.r, other.r),
                    g: interpolate(self.g, other.g),
                    b: interpolate(self.b, other.b)
                    }
            },
            Interpolation::Linear =>
            {
                let interpolate = |lhs, rhs|
                {
                    let diff = rhs as i32 - lhs as i32;
                    let result = lhs as f32 + diff as f32*amount;

                    result.round() as u8
                };

                Color{
                    r: interpolate(self.r, other.r),
                    g: interpolate(self.g, other.g),
                    b: interpolate(self.b, other.b)
                    }
            },
            Interpolation::Cubic =>
            {
                let interpolate = |lhs, rhs|
                {
                    todo!()
                };

                Color{
                    r: interpolate(self.r, other.r),
                    g: interpolate(self.g, other.g),
                    b: interpolate(self.b, other.b)
                    }
            }
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
            color.parse().map_err(|_| format!("error parsing {}", color))
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
    interpolation: Interpolation
}

impl Colorer
{
    pub fn new(colors: Vec<Color>, shift: bool, interpolation: Interpolation) -> Self
    {
        if colors.len()==0
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

        let mut out = Colorer{colors, shift, interpolation};
        out.word();

        out
    }

    pub fn word(&mut self)
    {
        if self.shift.is_some()
        {
            self.shift = Some(rand::random());
        }
    }

    pub fn color(&self, c: char, mut position: f32) -> String
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
            if let Some(amount) = self.shift
            {
                position += amount
            }

            if position>=1.0
            {
                position = position-1.0;
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
                color_position.floor() as usize,
                color_position.ceil() as usize,
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