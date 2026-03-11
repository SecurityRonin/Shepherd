import React from "react";

const App: React.FC = () => {
  return (
    <div className="h-screen w-screen flex flex-col bg-shepherd-bg">
      <header className="h-12 flex items-center px-4 border-b border-shepherd-border bg-shepherd-surface">
        <h1 className="text-sm font-semibold text-shepherd-text tracking-wide">
          SHEPHERD
        </h1>
      </header>
      <main className="flex-1 flex items-center justify-center text-shepherd-muted">
        <p>Loading...</p>
      </main>
    </div>
  );
};

export default App;
