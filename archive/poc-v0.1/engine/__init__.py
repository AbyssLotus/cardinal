"""Cardinal: world-agnostic persistent simulation engine.

Hard rule: this package never imports from `worlds/`. World content reaches
the engine only through `engine.core.registry` loading validated YAML.
"""

__version__ = "0.1.0"
