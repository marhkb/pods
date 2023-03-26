uniform sampler2D u_texture1;
uniform sampler2D u_texture2;

void
mainImage (out vec4 fragColor,
           in vec2  fragCoord,
           in vec2  resolution,
           in vec2  uv)
{
  vec4 source = GskTexture (u_texture1, uv);
  vec4 mask = GskTexture (u_texture2, uv);

  fragColor = source * (1.0 - mask.a);
}
