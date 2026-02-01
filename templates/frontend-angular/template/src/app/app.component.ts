import { Component } from '@angular/core';
import { RouterLink, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, RouterLink],
  template: `
    <header>
      <nav>
        <a routerLink="/">Home</a>
        <a routerLink="/about">About</a>
      </nav>
    </header>
    <main>
      <router-outlet></router-outlet>
    </main>
  `,
  styles: [`
    header {
      background: #1a1a2e;
      padding: 1rem 2rem;
      margin-bottom: 2rem;
    }
    nav {
      display: flex;
      gap: 1rem;
    }
    nav a {
      color: #dd0031;
      font-weight: 500;
    }
    nav a:hover {
      color: #fff;
    }
    main {
      max-width: 1200px;
      margin: 0 auto;
      padding: 0 2rem;
    }
  `]
})
export class AppComponent {
  title = '{{project_name}}';
}
