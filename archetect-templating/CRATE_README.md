# Archetect Templating Engine

This crate is a vendoring of the excellent [minijinja](https://github.com/mitsuhiko/minijinja) templating engine.

This vendored version of minijinja __is inferior__ to the actual minijinja project, and is strictly intended for exclusive
use within Archetect due to very specific requirements not present in other projects.

## Vendoring Justification

A central concept in Archetect is to _pre-shape_ variables to have the same case style as the value they contain.  Examples:

```
    ProjectName: CustomerService
    projectName: customerService
    project-name: customer-service
    project_name: customer-service
```

This allows archetype authors to use the correctly shaped variables throughout the project, including within path names.

Example:

```
public class ExampleFacade {

    private final {{ ProjectName }} {{ projectName }};
    
    public ExampleFacade({{ ProjectName }} {{ projectName }}) {
        this.{{ projectName }} = {{ projectName }};
    }
}
```

Resulting in:

```
public class ExampleFacade {

    private final CustomerService customerService; 
    
    public ExampleFacade(CustomerService customerService) {
        this.customerService = customerService;
    }
}
```

To allow the most natural use of this concept, this vendored version of minijinja allows for variables like `project-name`,
which would typically be a mathematical subtraction between two variables in Jinja.  In the case of Archetect's templating flavor,
`project-name` is the name of a variable.  To perform subtractions in Archetect templates, don't be a savage - use spaces 
before and after the subtraction sign like any sane human being should.  

To perform math in Archetect templates:

Don't:
```
    {{total-discount}} // This would be a variable
```

Do:
```
    {{total - discount}} // This would be subtraction
    {{total -discount}} // This works, too, but don't be an idiot 😉
```