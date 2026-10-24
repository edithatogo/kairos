#![forbid(unsafe_code)]

use geo::{
    line_measures::InterpolatePoint, Bearing, Distance, Euclidean, Geodesic, Haversine, Point,
};

/// Declared coordinate system for spatial geometric calculations.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CoordinateSystem {
    /// WGS84 standard geographic latitude and longitude (in degrees).
    GeographicWgs84,
    /// Projected 2D Cartesian planar grid (in meters).
    ProjectedPlanar { projection_name: String },
}

/// Errors occurring during geometric transformations and route calculations.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GeometryError {
    #[error("invalid geographic coordinate (lat: {lat}, lon: {lon}): {reason}")]
    InvalidCoordinate {
        lat: f64,
        lon: f64,
        reason: &'static str,
    },

    #[error("invalid projected planar coordinate (x: {x}, y: {y}): {reason}")]
    InvalidProjectedCoordinate {
        x: f64,
        y: f64,
        reason: &'static str,
    },

    #[error("negative distance provided for route interpolation: {0} meters")]
    NegativeDistance(f64),

    #[error("normalized progress parameter out of range [0.0, 1.0]: {0}")]
    NormalizedProgressOutOfRange(f64),

    #[error("route contains zero waypoints")]
    EmptyRoute,

    #[error("invalid geometric parameter: {0}")]
    InvalidParameter(&'static str),
}

/// Validated WGS84 geographic coordinate (latitude and longitude in degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoCoordinate {
    lat: f64,
    lon: f64,
}

impl GeoCoordinate {
    /// Constructs and validates a WGS84 geographic coordinate.
    pub fn try_new(lat: f64, lon: f64) -> Result<Self, GeometryError> {
        if !lat.is_finite() {
            return Err(GeometryError::InvalidCoordinate {
                lat,
                lon,
                reason: "latitude must be a finite number",
            });
        }
        if !lon.is_finite() {
            return Err(GeometryError::InvalidCoordinate {
                lat,
                lon,
                reason: "longitude must be a finite number",
            });
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(GeometryError::InvalidCoordinate {
                lat,
                lon,
                reason: "latitude must be between -90.0 and +90.0 degrees",
            });
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(GeometryError::InvalidCoordinate {
                lat,
                lon,
                reason: "longitude must be between -180.0 and +180.0 degrees",
            });
        }
        Ok(Self { lat, lon })
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lon(&self) -> f64 {
        self.lon
    }

    /// Converts to a `geo::Point` using standard GIS convention (x = lon, y = lat).
    pub fn to_point(&self) -> Point<f64> {
        Point::new(self.lon, self.lat)
    }

    /// Constructs from a `geo::Point` where x = lon and y = lat.
    pub fn from_point(p: Point<f64>) -> Result<Self, GeometryError> {
        Self::try_new(p.y(), p.x())
    }

    /// Computes the spherical Haversine distance in meters to another coordinate.
    pub fn haversine_distance_to(&self, other: &Self) -> f64 {
        Haversine.distance(self.to_point(), other.to_point())
    }

    /// Computes the exact ellipsoidal WGS84 geodesic distance in meters to another coordinate.
    pub fn geodesic_distance_to(&self, other: &Self) -> f64 {
        Geodesic.distance(self.to_point(), other.to_point())
    }

    /// Computes the spherical initial bearing in degrees (0..360) towards another coordinate.
    pub fn haversine_bearing_to(&self, other: &Self) -> f64 {
        Haversine.bearing(self.to_point(), other.to_point())
    }

    /// Computes the ellipsoidal WGS84 initial bearing in degrees towards another coordinate.
    pub fn geodesic_bearing_to(&self, other: &Self) -> f64 {
        Geodesic.bearing(self.to_point(), other.to_point())
    }
}

/// Validated Cartesian planar coordinate in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedCoordinate {
    x: f64,
    y: f64,
}

impl ProjectedCoordinate {
    pub fn try_new(x: f64, y: f64) -> Result<Self, GeometryError> {
        if !x.is_finite() {
            return Err(GeometryError::InvalidProjectedCoordinate {
                x,
                y,
                reason: "x coordinate must be a finite number",
            });
        }
        if !y.is_finite() {
            return Err(GeometryError::InvalidProjectedCoordinate {
                x,
                y,
                reason: "y coordinate must be a finite number",
            });
        }
        Ok(Self { x, y })
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn to_point(&self) -> Point<f64> {
        Point::new(self.x, self.y)
    }

    pub fn from_point(p: Point<f64>) -> Result<Self, GeometryError> {
        Self::try_new(p.x(), p.y())
    }

    pub fn euclidean_distance_to(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }
}

/// Local tangent plane (equirectangular planar approximation) around an anchor origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalTangentPlaneProjection {
    origin: GeoCoordinate,
    lat_scale: f64,
    lon_scale: f64,
}

impl LocalTangentPlaneProjection {
    // Mean Earth radius in meters according to WGS84 standard
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

    pub fn new(origin: GeoCoordinate) -> Self {
        let rad = origin.lat.to_radians();
        let lat_scale = Self::EARTH_RADIUS_METERS * std::f64::consts::PI / 180.0;
        let lon_scale = Self::EARTH_RADIUS_METERS * rad.cos() * std::f64::consts::PI / 180.0;
        Self {
            origin,
            lat_scale,
            lon_scale,
        }
    }

    pub fn origin(&self) -> GeoCoordinate {
        self.origin
    }

    /// Projects a geographic WGS84 coordinate into local planar meters (x = East, y = North).
    pub fn project(&self, coord: &GeoCoordinate) -> ProjectedCoordinate {
        let dx = (coord.lon - self.origin.lon) * self.lon_scale;
        let dy = (coord.lat - self.origin.lat) * self.lat_scale;
        ProjectedCoordinate { x: dx, y: dy }
    }

    /// Inverts a local planar offset back into a geographic WGS84 coordinate.
    pub fn unproject(&self, proj: &ProjectedCoordinate) -> Result<GeoCoordinate, GeometryError> {
        let dlat = proj.y / self.lat_scale;
        let dlon = if self.lon_scale.abs() > 1e-12 {
            proj.x / self.lon_scale
        } else {
            0.0
        };
        GeoCoordinate::try_new(self.origin.lat + dlat, self.origin.lon + dlon)
    }
}

/// Multi-segment geographic route composed of WGS84 coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoRoute {
    waypoints: Vec<GeoCoordinate>,
}

impl GeoRoute {
    pub fn try_new(waypoints: Vec<GeoCoordinate>) -> Result<Self, GeometryError> {
        if waypoints.is_empty() {
            return Err(GeometryError::EmptyRoute);
        }
        Ok(Self { waypoints })
    }

    pub fn waypoints(&self) -> &[GeoCoordinate] {
        &self.waypoints
    }

    pub fn waypoint_count(&self) -> usize {
        self.waypoints.len()
    }

    /// Computes the total spherical Haversine path distance in meters.
    pub fn total_distance_haversine(&self) -> f64 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for window in self.waypoints.windows(2) {
            total += window[0].haversine_distance_to(&window[1]);
        }
        total
    }

    /// Computes the total ellipsoidal WGS84 geodesic path distance in meters.
    pub fn total_distance_geodesic(&self) -> f64 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for window in self.waypoints.windows(2) {
            total += window[0].geodesic_distance_to(&window[1]);
        }
        total
    }

    /// Interpolates the exact geographic coordinate at a given distance along the path.
    pub fn interpolate_distance(
        &self,
        distance_meters: f64,
    ) -> Result<GeoCoordinate, GeometryError> {
        if !distance_meters.is_finite() {
            return Err(GeometryError::InvalidParameter("distance must be finite"));
        }
        if distance_meters < 0.0 {
            return Err(GeometryError::NegativeDistance(distance_meters));
        }
        if self.waypoints.len() == 1 || distance_meters == 0.0 {
            return Ok(self.waypoints[0]);
        }

        let total_dist = self.total_distance_haversine();
        if distance_meters >= total_dist {
            return Ok(*self.waypoints.last().unwrap());
        }

        let mut accumulated = 0.0;
        for window in self.waypoints.windows(2) {
            let seg_dist = window[0].haversine_distance_to(&window[1]);
            if seg_dist == 0.0 {
                continue;
            }
            if accumulated + seg_dist >= distance_meters {
                let remaining = distance_meters - accumulated;
                let ratio = (remaining / seg_dist).clamp(0.0, 1.0);
                let p1 = window[0].to_point();
                let p2 = window[1].to_point();
                let interp = Haversine.point_at_ratio_between(p1, p2, ratio);
                return GeoCoordinate::from_point(interp);
            }
            accumulated += seg_dist;
        }

        Ok(*self.waypoints.last().unwrap())
    }

    /// Interpolates the geographic coordinate at normalized progress parameter $t \in [0.0, 1.0]$.
    pub fn interpolate_normalized(&self, progress: f64) -> Result<GeoCoordinate, GeometryError> {
        if !progress.is_finite() {
            return Err(GeometryError::InvalidParameter("progress must be finite"));
        }
        if !(0.0..=1.0).contains(&progress) {
            return Err(GeometryError::NormalizedProgressOutOfRange(progress));
        }
        let total_dist = self.total_distance_haversine();
        self.interpolate_distance(progress * total_dist)
    }
}

/// Multi-segment planar Cartesian route composed of projected coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedRoute {
    waypoints: Vec<ProjectedCoordinate>,
}

impl ProjectedRoute {
    pub fn try_new(waypoints: Vec<ProjectedCoordinate>) -> Result<Self, GeometryError> {
        if waypoints.is_empty() {
            return Err(GeometryError::EmptyRoute);
        }
        Ok(Self { waypoints })
    }

    pub fn waypoints(&self) -> &[ProjectedCoordinate] {
        &self.waypoints
    }

    pub fn waypoint_count(&self) -> usize {
        self.waypoints.len()
    }

    pub fn total_length(&self) -> f64 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for window in self.waypoints.windows(2) {
            total += window[0].euclidean_distance_to(&window[1]);
        }
        total
    }

    pub fn interpolate_distance(
        &self,
        distance: f64,
    ) -> Result<ProjectedCoordinate, GeometryError> {
        if !distance.is_finite() {
            return Err(GeometryError::InvalidParameter("distance must be finite"));
        }
        if distance < 0.0 {
            return Err(GeometryError::NegativeDistance(distance));
        }
        if self.waypoints.len() == 1 || distance == 0.0 {
            return Ok(self.waypoints[0]);
        }

        let total_len = self.total_length();
        if distance >= total_len {
            return Ok(*self.waypoints.last().unwrap());
        }

        let mut accumulated = 0.0;
        for window in self.waypoints.windows(2) {
            let seg_len = window[0].euclidean_distance_to(&window[1]);
            if seg_len == 0.0 {
                continue;
            }
            if accumulated + seg_len >= distance {
                let remaining = distance - accumulated;
                let ratio = (remaining / seg_len).clamp(0.0, 1.0);
                let p1 = window[0].to_point();
                let p2 = window[1].to_point();
                let interp = Euclidean.point_at_ratio_between(p1, p2, ratio);
                return ProjectedCoordinate::from_point(interp);
            }
            accumulated += seg_len;
        }

        Ok(*self.waypoints.last().unwrap())
    }

    pub fn interpolate_normalized(
        &self,
        progress: f64,
    ) -> Result<ProjectedCoordinate, GeometryError> {
        if !progress.is_finite() {
            return Err(GeometryError::InvalidParameter("progress must be finite"));
        }
        if !(0.0..=1.0).contains(&progress) {
            return Err(GeometryError::NormalizedProgressOutOfRange(progress));
        }
        let total_len = self.total_length();
        self.interpolate_distance(progress * total_len)
    }
}
