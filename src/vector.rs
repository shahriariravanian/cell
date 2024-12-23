use std::ops::{Add, Sub, Mul, Div, Deref, DerefMut};
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};

#[derive(Debug, Clone, PartialEq)]
pub struct Vector (pub Vec<f64>);

impl Vector {
    fn new(v: Vec<f64>) -> Vector {
        Vector(v)
    }
    
    fn sum(&self) -> f64 {
        self.iter().sum()
    }
    
    fn product(&self) -> f64 {
        self.iter().product()
    }
    
    fn mapv(&self, f: impl Fn (f64) -> f64) -> Vector {
        Vector(self.iter().map(|x| f(*x)).collect())
    }
}

/**************** Deref *********************/

impl Deref for Vector {
    type Target = Vec<f64>;
    
    fn deref(&self) -> &Vec<f64> {
        &self.0
    }
}

impl DerefMut for Vector {
    fn deref_mut(&mut self) -> &mut Vec<f64> {
        &mut self.0
    }
}

/*
*   There are multiple forms of each operator:
*   A @ B (where @ is any operator) consumes both operands and uses A for storage
*   A & &B consumes A and uses it as the storage
*   &A @ &B allocates a new vector for the result
*   &A @ B consumes B and uses it for storage
*/

/**************** Add **********************/

impl Add<Vector> for Vector {
    type Output = Vector;
    
    fn add(mut self, rhs: Self) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] += rhs[i];    
        };
        
        self
    }
}

impl Add<&Vector> for Vector {
    type Output = Vector;
    
    fn add(mut self, rhs: &Vector) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] += rhs[i];    
        };
        
        self
    }
}

impl<'a> Add<&'a Vector> for &'a Vector {
    type Output = Vector;
    
    fn add(self, rhs: &'a Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] += rhs[i];    
        };
        
        v
    }
}

// note: rhs is consumed
impl Add<Vector> for &Vector {
    type Output = Vector;
    
    fn add(self, mut rhs: Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            rhs[i] += self[i];    
        };
        
        rhs
    }
}

impl Add<f64> for Vector {
    type Output = Vector;
    
    fn add(mut self, rhs: f64) -> Vector {
    
        for i in 0..self.len() {
            self[i] += rhs;    
        };
        
        self
    }
}

impl Add<f64> for &Vector {
    type Output = Vector;
    
    fn add(self, rhs: f64) -> Vector {
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] += rhs;    
        };
        
        v
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Vector) {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] += rhs[i];    
        };        
    }
}


/**************** Sub **********************/

impl Sub<Vector> for Vector {
    type Output = Vector;
    
    fn sub(mut self, rhs: Self) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] -= rhs[i];    
        };
        
        self
    }
}

impl Sub<&Vector> for Vector {
    type Output = Vector;
    
    fn sub(mut self, rhs: &Vector) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] -= rhs[i];
        };
        
        self
    }
}

impl<'a> Sub<&'a Vector> for &'a Vector {
    type Output = Vector;
    
    fn sub(self, rhs: &'a Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] -= rhs[i];
        };
        
        v
    }
}

// note: rhs is consumed
impl Sub<Vector> for &Vector {
    type Output = Vector;
    
    fn sub(self, mut rhs: Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            rhs[i] -= self[i];
        };
        
        rhs
    }
}

impl Sub<f64> for Vector {
    type Output = Vector;
    
    fn sub(mut self, rhs: f64) -> Vector {
    
        for i in 0..self.len() {
            self[i] -= rhs;    
        };
        
        self
    }
}

impl Sub<f64> for &Vector {
    type Output = Vector;
    
    fn sub(self, rhs: f64) -> Vector {
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] -= rhs;    
        };
        
        v
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Vector) {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] -= rhs[i];    
        };        
    }
}


/********************** Mul ***************************/

impl Mul<Vector> for Vector {
    type Output = Vector;
    
    fn mul(mut self, rhs: Self) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] *= rhs[i];    
        };
        
        self
    }
}

impl Mul<&Vector> for Vector {
    type Output = Vector;
    
    fn mul(mut self, rhs: &Vector) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] *= rhs[i];    
        };
        
        self
    }
}

impl<'a> Mul<&'a Vector> for &'a Vector {
    type Output = Vector;
    
    fn mul(self, rhs: &'a Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] *= rhs[i];    
        };
        
        v
    }
}

// note: rhs is consumed
impl Mul<Vector> for &Vector {
    type Output = Vector;
    
    fn mul(self, mut rhs: Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            rhs[i] *= self[i];
        };
        
        rhs
    }
}

impl Mul<f64> for Vector {
    type Output = Vector;
    
    fn mul(mut self, rhs: f64) -> Vector {
    
        for i in 0..self.len() {
            self[i] *= rhs;    
        };
        
        self
    }
}

impl Mul<f64> for &Vector {
    type Output = Vector;
    
    fn mul(self, rhs: f64) -> Vector {
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] *= rhs    
        };
        
        v
    }
}

impl MulAssign for Vector {
    fn mul_assign(&mut self, rhs: Vector) {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] *= rhs[i];    
        };        
    }
}

impl Vector {
    fn dot(&self, v: &Vector) -> f64 {
        assert_eq!(self.len(), v.len());
        self.iter().zip(v.iter()).map(|(x,y)| x*y).sum()
    }
}

/********************** Div ***************************/

impl Div<Vector> for Vector {
    type Output = Vector;
    
    fn div(mut self, rhs: Self) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] /= rhs[i];    
        };
        
        self
    }
}

impl Div<&Vector> for Vector {
    type Output = Vector;
    
    fn div(mut self, rhs: &Vector) -> Self {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] /= rhs[i];
        };
        
        self
    }
}

impl<'a> Div<&'a Vector> for &'a Vector {
    type Output = Vector;
    
    fn div(self, rhs: &'a Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] /= rhs[i];    
        };
        
        v
    }
}

// note: rhs is consumed
impl Div<Vector> for &Vector {
    type Output = Vector;
    
    fn div(self, mut rhs: Vector) -> Vector {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            rhs[i] /= self[i];
        };
        
        rhs
    }
}

impl Div<f64> for Vector {
    type Output = Vector;
    
    fn div(mut self, rhs: f64) -> Vector {
    
        for i in 0..self.len() {
            self[i] /= rhs;    
        };
        
        self
    }
}

impl Div<f64> for &Vector {
    type Output = Vector;
    
    fn div(self, rhs: f64) -> Vector {
        let mut v = self.clone();
        
        for i in 0..self.len() {
            v[i] /= rhs;    
        };
        
        v
    }
}

impl DivAssign for Vector {
    fn div_assign(&mut self, rhs: Vector) {
        assert_eq!(self.len(), rhs.len());
        
        for i in 0..self.len() {
            self[i] /= rhs[i];    
        };        
    }
}


/***************************************************/

#[test]
fn test_vector() {
    let v = Vector::new(vec![10.0, 8.0, 5.0]);
    let w = Vector::new(vec![2.0, 4.0, 5.0]);
    
    assert_eq!(&v + &w, Vector::new(vec![12.0, 12.0, 10.0]));
    assert_eq!(&v - &w, Vector::new(vec![8.0, 4.0, 0.0]));
    assert_eq!(&v * &w, Vector::new(vec![20.0, 32.0, 25.0]));
    assert_eq!(&v / &w, Vector::new(vec![5.0, 2.0, 1.0]));
    assert_eq!(v / w, Vector::new(vec![5.0, 2.0, 1.0]));
}


