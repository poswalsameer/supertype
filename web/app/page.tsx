import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { ValueStrip } from "@/components/ValueStrip";
import { ProblemSolution } from "@/components/ProblemSolution";
import { HowItWorks } from "@/components/HowItWorks";
import { ModelShowcase } from "@/components/ModelShowcase";
import { PrivacySection } from "@/components/PrivacySection";
import { WorkflowShowcase } from "@/components/WorkflowShowcase";
import { PerformanceSection } from "@/components/PerformanceSection";
import { FinalCTA } from "@/components/FinalCTA";
import { Footer } from "@/components/Footer";

export default function Home() {
  return (
    <div id="top" className="flex flex-col min-h-full">
      <Navbar />
      <main id="main" className="flex-1">
        <Hero />
        <ValueStrip />
        <ProblemSolution />
        <HowItWorks />
        <ModelShowcase />
        <PrivacySection />
        <WorkflowShowcase />
        <PerformanceSection />
        <FinalCTA />
      </main>
      <Footer />
    </div>
  );
}
