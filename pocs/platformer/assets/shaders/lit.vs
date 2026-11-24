#version 330

// Skyhop lit pass, vertex stage. Works for raylib's batch (DrawCube, DrawSphere, DrawCubeWires)
// and for meshes (DrawModelEx): both set mvp, matModel, matNormal and matView themselves.

in vec3 vertexPosition;
in vec2 vertexTexCoord;
in vec3 vertexNormal;
in vec4 vertexColor;

uniform mat4 mvp;
uniform mat4 matModel;
uniform mat4 matNormal;
uniform mat4 matView;

out vec3 fragPosition;
out vec2 fragTexCoord;
out vec4 fragColor;
out vec3 fragNormal;
out vec3 fragEye;
out float fragDepth;

void main()
{
    vec4 world = matModel*vec4(vertexPosition, 1.0);
    fragPosition = world.xyz;
    fragTexCoord = vertexTexCoord;
    fragColor = vertexColor;
    fragNormal = mat3(matNormal)*vertexNormal;
    // The camera position, from the rigid view matrix.
    fragEye = -(transpose(mat3(matView))*matView[3].xyz);
    gl_Position = mvp*vec4(vertexPosition, 1.0);
    fragDepth = gl_Position.w;
}
