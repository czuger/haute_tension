Rails.application.routes.draw do
  resources :aventures do
    get :reroll
    get :read
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
  end

  resources :books
  resources :page_links
  resources :pages
  # For details on the DSL available within this file, see http://guides.rubyonrails.org/routing.html

  root 'aventures#index'
end
