use std::fmt;


struct Color
{
    r: u8,
    g: u8,
    b: u8
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
    frequency: f32
}

impl Colorer
{
    pub fn new(frequency: f32) -> Self
    {
        Colorer{frequency}
    }

    fn single_height(&self, position: f32) -> u8
    {
        (position.sin()*255.0).round() as u8
    }

    pub fn color(&self, c: char, position: f32) -> String
    {
        if c==' '
        {
            return c.to_string();
        }

        const DIST: f32 = 1.0/3.0;

        let color = Color{
            r: self.single_height((0.0+position)*self.frequency),
            g: self.single_height((DIST+position)*self.frequency),
            b: self.single_height((DIST*2.0+position)*self.frequency)
            };

        format!("[c/{color}:{c}]")
    }
}