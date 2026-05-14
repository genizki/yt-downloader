interface Props {
  hue: number;
  img: string;
  duration: string;
}

export function Thumb({ hue, img, duration }: Props) {
  return (
    <div className="thumb">
      <img src={img} width="200" />
      <span className="thumb-duration">{duration}</span>
    </div>
  );
}
