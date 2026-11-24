#version 330

// Skyhop lit pass, fragment stage: a fixed sun, hemisphere ambient, a soft rim, distance fog to
// the sky colour, terrain detail on axis-aligned flat faces, and a shimmer on glass materials.
// Colours are display (sRGB) values throughout; nothing here is linearised.

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;
in vec3 fragEye;
in float fragDepth;

uniform sampler2D texture0;
uniform vec4 colDiffuse;
// Simulation tick count as a float (an exact integer, 60 ticks per second).
uniform float uTime;

out vec4 finalColor;

const vec3 SKY = vec3(135.0, 206.0, 235.0)/255.0;
const vec3 SUN_DIR = vec3(0.4472136, 0.7826238, 0.4323065);   // normalize(0.4, 0.7, 0.3867)
const vec3 SUN = vec3(1.00, 0.95, 0.84);
const vec3 AMBIENT_SKY = vec3(0.66, 0.78, 0.94);
const vec3 AMBIENT_GROUND = vec3(0.46, 0.40, 0.36);
const float FOG_NEAR = 22.0;
const float FOG_FAR = 70.0;
const float FLAT = 0.9995;

void main()
{
    float seconds = uTime/60.0;

    vec4 texel = texture(texture0, fragTexCoord);
    // A glass material arrives with colDiffuse.a < 1 (the gem). It is drawn opaque and shimmers.
    float glass = 1.0 - step(0.99, colDiffuse.a);
    vec3 base = texel.rgb*colDiffuse.rgb*fragColor.rgb;
    float alpha = texel.a*fragColor.a;

    vec3 v = normalize(fragEye - fragPosition);

    // The supplied normal, or the face's own when the two disagree by more than 45 degrees:
    // DrawSphere sets no normals, so its vertices carry whatever the batch saw last.
    float normalLength = length(fragNormal);
    vec3 n = (normalLength > 0.0001)? fragNormal/normalLength : vec3(0.0, 1.0, 0.0);
    vec3 face = cross(dFdx(fragPosition), dFdy(fragPosition));
    float faceLength = length(face);
    float agrees = 1.0;
    if (faceLength > 1.0e-12)
    {
        face /= faceLength;
        if (dot(face, v) < 0.0) face = -face;
        agrees = dot(face, n);
        if (agrees < 0.7) n = face;
    }

    // Terrain detail: only flat faces whose normal is exactly a world axis (raylib cubes), never
    // the smooth models. Tops get a one-metre checker and a slow wind ripple, sides get strata.
    float terrain = (1.0 - glass)*step(0.999, agrees);
    float top = terrain*step(FLAT, n.y);
    float side = terrain*step(FLAT, max(abs(n.x), abs(n.z)));
    vec3 p = fragPosition;
    float checker = mod(floor(p.x) + floor(p.z), 2.0);
    float ripple = 0.030*sin(p.x*2.6 + p.z*1.7 - seconds*1.8) + 0.018*sin(p.x*9.0 - p.z*7.0 + seconds*2.3);
    float strata = step(0.5, fract(p.y*2.0));
    base *= 1.0 + top*((checker - 0.5)*0.10 + ripple);
    base *= 1.0 - side*(0.08 + 0.06*strata);

    // Sun and hemisphere ambient.
    float ndl = dot(n, SUN_DIR);
    float sun = 0.78*max(ndl, 0.0) + 0.22*(ndl*0.5 + 0.5);
    vec3 ambient = mix(AMBIENT_GROUND, AMBIENT_SKY, n.y*0.5 + 0.5);
    vec3 color = base*(ambient*0.74 + SUN*sun*0.52);

    // Soft rim toward the sky, stronger on up-facing parts.
    float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0);
    color += rim*(0.10 + 0.14*(n.y*0.5 + 0.5))*AMBIENT_SKY*(1.0 - top);

    // Gem shimmer: a sharp highlight plus a bright wave that travels across the facets.
    vec3 h = normalize(SUN_DIR + v);
    float sparkle = pow(max(dot(n, h), 0.0), 24.0);
    float wave = 0.5 + 0.5*sin(seconds*3.2 + dot(p, vec3(5.0, 7.0, 3.0)));
    color += glass*(0.55*sparkle + 0.22*wave*wave)*vec3(1.0, 0.97, 0.85);

    // Distance fog toward the sky.
    float fog = smoothstep(FOG_NEAR, FOG_FAR, fragDepth)*0.9;
    color = mix(color, SKY, fog);

    finalColor = vec4(clamp(color, 0.0, 1.0), alpha);
}
