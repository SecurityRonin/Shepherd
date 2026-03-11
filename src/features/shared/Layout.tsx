import React from "react";
import { Header } from "./Header";

interface LayoutProps {
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({ children }) => {
  return (
    <div className="h-screen w-screen flex flex-col bg-shepherd-bg overflow-hidden">
      <Header />
      <main className="flex-1 overflow-hidden">{children}</main>
    </div>
  );
};
