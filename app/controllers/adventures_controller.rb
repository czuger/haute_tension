class AdventuresController < ApplicationController

  before_action :set_adventure, except: [:new, :create]
  before_action :set_new_adventure, only: [:new, :create]

  def welcome
  end

  # GET /aventures
  # GET /adventures.json
  def list
    @adventures = @user&.adventures
    @adventures ||= Adventure.none
  end

  # GET /adventures/1
  # GET /adventures/1.json
  def show
  end

  def play
    if @user
      @page = @adventure.current_page
    else
      @page = Page.find_by_page_hash( params[:page_id] )
    end

    # @page.text.each{ |text| text.gsub!( '%ADVENTURE_ID%', params[:adventure_id].to_i.to_s ) }
  end

  def read_choice
    if @user
      page = Page.find_by_page_hash( params[:page_id] )
      @adventure.current_page = page
      ActiveRecord::Base.transaction do
        @adventure.game_logs.create!( page: @adventure.current_page, log_type: GameLog::JOURNEY )
        @adventure.save!
      end
      redirect_to play_adventures_path
    else
      redirect_to play_adventures_path( page_id: params[:page_id] )
    end
  end

  # GET /adventures/new
  def new
  end

  def start_book
    book = Book.find_by_book_key(params[ :book_key ] )

    if @user
      @adventure = Adventure.new( user: @user, book: book, current_page: book.first_page, items: {} )
      roll_adventure

      if @adventure.save
        @user.update!( current_adventure_id: @adventure.id )
        @adventure.game_logs.create!( page: @adventure.current_page, log_type: GameLog::JOURNEY )

        redirect_to adventures_path, notice: 'Aventure crée.'
      else
        render :new
      end
    else
      redirect_to play_adventures_path( page_id: book.first_page.page_hash )
    end
  end

  def reroll
    roll_adventure
    @adventure.save!
    render :show
  end

  def roll_dices
    @dices = Hazard.s2d6
  end

  # DELETE /adventures/1
  # DELETE /adventures/1.json
  def destroy
    ActiveRecord::Base.transaction do
      @user.update!( current_adventure_id: nil )
      @adventure.destroy
    end

    respond_to do |format|
      format.html { redirect_to list_adventures_url, notice: 'Aventure supprimée avec succès.' }
      format.json { head :no_content }
    end
  end

  private
    # Use callbacks to share common setup or constraints between actions.

    # Never trust parameters from the scary internet, only allow the white list through.
    def adventure_params
      params.require(:adventure).permit( :book_id )
    end

    def roll_adventure
      difficulty = params[:difficulty] || 'medium'

      case difficulty
        when 'medium'
          roll_regular
        when 'easy'
          roll_easy
        when 'hard'
          roll_hardcore
        else
          raise "Unknown difficulty #{difficulty.inspect}"
      end

      @adventure.hp_max = @adventure.hp
      @adventure.strength = 19 if @adventure.strength == 17
      @adventure.strength = 20 if @adventure.strength == 18
      @adventure.strength_max = @adventure.strength
    end

    def roll_regular
      @adventure.hp = 18 + Hazard.r2d6
      @adventure.strength = 6 + Hazard.r2d6
      @adventure.gold = Hazard.r4d6
    end

    def roll_easy
      @adventure.hp = 18 + 6 + Hazard.d6
      @adventure.strength = 6 + 6 + Hazard.d6
      @adventure.gold = 12 + Hazard.r2d6
    end

    def roll_hardcore
      @adventure.hp = 18 + 1 + Hazard.d6
      @adventure.strength = 6 + 1 + Hazard.d6
      @adventure.gold = 4 + Hazard.r2d6
    end

    def set_new_adventure
      set_user
      @adventure = Adventure.new
      @books = Book.all.order( :name )
    end

end
