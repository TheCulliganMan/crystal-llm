declare module "mermaid" {
  export type MermaidRenderResult = {
    svg: string;
    bindFunctions?: (element: Element) => void;
  };

  export type MermaidConfig = {
    startOnLoad?: boolean;
    securityLevel?: "strict" | "loose" | "antiscript" | "sandbox";
    theme?: string;
    flowchart?: {
      htmlLabels?: boolean;
    };
    themeVariables?: Record<string, string>;
  };

  export interface MermaidApi {
    initialize: (config: MermaidConfig) => void;
    render: (id: string, text: string) => Promise<MermaidRenderResult>;
  }

  const mermaid: MermaidApi;
  export default mermaid;
}
