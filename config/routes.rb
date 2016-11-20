Rails.application.routes.draw do

  resources :adventures do
    get :reroll
    get :play
    get 'read_choice/:page_id', action: :read_choice, as: :read_choice
    get :roll_dices
    get :die
    get :log
  end

  resources :notes, only: [ :edit, :update ]

  resources :fights, only: [ :show, :update ] do
    patch :add_monster
    patch :remove_monster
    patch :fight_monster
  end

  resources :pages, except: [ :new, :create ]
  # For details on the DSL available within this file, see http://guides.rubyonrails.org/routing.html

  root 'adventures#index'
end
