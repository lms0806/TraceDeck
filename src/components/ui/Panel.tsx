import type { HTMLAttributes, ReactNode } from "react";

type PanelProps = HTMLAttributes<HTMLElement> & {
  as?: "div" | "section";
  children: ReactNode;
};

export function Panel({
  as: Element = "section",
  children,
  className = "",
  ...props
}: PanelProps) {
  return (
    <Element
      {...props}
      className={`rounded-lg border border-[#2d3d4c] bg-[#17212b] p-[17px] [&>header]:mb-[13px] [&>header]:flex [&>header]:justify-between [&>header>h2]:m-0 [&>header>h2]:text-sm [&>header>h2]:text-[#eef6fd] [&>header>span]:text-[11px] [&>header>span]:text-[#7890a4] ${className}`}
    >
      {children}
    </Element>
  );
}
